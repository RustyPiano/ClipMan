# 0002 剪贴板采集：一次变化只保存一条记录

- 状态：已采纳

## 背景

一次复制通常同时提供多种格式。访达复制文件时还会附带文件名文本和图标图片。如果分别读取文本和图片，一次复制会生成多条记录。

## 决定

- `clipboard::read_clipboard_snapshot` 每次变化只读取一个代表格式，优先级为 Files > Text（附带可选 HTML）> Image。图片解码要复制整张位图，只在没有文件和文本时才尝试。
- `clipboard::start_monitor` 把 `Master::new` 的错误返回给调用方。`run()` 只在系统剪贴板不可用时返回错误，这时应用无法工作，监听线程直接 panic。
- 采集前先记下剪贴板序号，再检查隐私标记、读取内容；读取后序号变化就放弃本次采集，保证标记和内容来自同一次复制（新复制会触发自己的采集）。Windows 打开剪贴板检查标记重试 5 次仍失败时跳过本次采集。
- X11 没有剪贴板序号，ClipLeaf 用 XFixes 自己计数：一个常驻连接订阅 `CLIPBOARD` 的所有者变化事件（`X11SelectionWatch`），每次读取序号前先做一次往返请求，确保之前的事件都已到达，再累加收到的事件数。同一个应用再次复制也会重新设置所有者，所以能区分连续两次复制。开启“忽略密码类剪贴板”时拿不到序号就跳过本次采集，因为无法确认标记和内容属于同一次复制。
- Linux 在 X11 下检查隐私标记：先读 `CLIPBOARD` 的 `TARGETS`，含 `x-kde-passwordManagerHint` 时再读它的值，值为 `secret` 就跳过本次采集。KeePassXC 复制密码时会设置这个标记。项目在 Linux 上只通过 X11 读剪贴板（arboard 没有开启 Wayland 支持，clipboard-master 只支持 X11），所以没有单独的 Wayland 检查。连不上 X 服务器、剪贴板所有者拒绝提供 `TARGETS`、1 秒内没有响应或用 INCR 分段传输时都返回错误，按无法检查处理，跳过本次采集。
- `Handler::sleep_interval` 设为 100ms，替换 clipboard-master 默认的 500ms。macOS 按这个间隔轮询 `changeCount`；X11 用它作为两次检查事件之间的等待，每次采集后还要再等一个间隔才重新监听，这段时间里的新复制会被漏掉。Windows 由 `WM_CLIPBOARDUPDATE` 消息驱动，不使用这个值。
- 来源应用在分派前同步读取。图片在后台任务里处理，来源必须在此之前确定。
- HTML 超过 `max_text_bytes` 时只丢弃 HTML，纯文本照常保存（`capture_text`）。
- `save_to_storage` 只在插入失败时返回 false。新记录因历史上限被立即裁掉（`Ok(None)`）也算保存成功，globalPlain 是否剥离格式不受历史上限影响。
- 延迟处理的旧图片可能已被裁掉，这时不发出事件，界面不会出现不存在的行。
- 每次采集后托盘菜单从数据库重建。新菜单完整构建后才替换；构建失败时保留旧菜单（`tray::update_tray_menu`）。

## 影响

不能恢复对文本和图片的独立读取，否则会重新出现一次复制两条记录的问题。
