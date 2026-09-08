// macOS-only native QA helper; invoked by run.py. Never bypasses TCC.
import AppKit
import ApplicationServices

let args = CommandLine.arguments
func app(_ id: String) -> NSRunningApplication {
  NSRunningApplication.runningApplications(withBundleIdentifier: id).first!
}
func attr(_ e: AXUIElement, _ name: String) -> CFTypeRef? {
  var v: CFTypeRef?
  AXUIElementCopyAttributeValue(e, name as CFString, &v)
  return v
}
func tree(_ e: AXUIElement, _ depth: Int = 0) {
  if depth > 12 { return }
  if let title = attr(e, "AXTitle") as? String, !title.isEmpty { print(title) }
  for c in (attr(e, "AXChildren") as? [AXUIElement] ?? []).prefix(100) { tree(c, depth + 1) }
}
func key(_ code: UInt16, _ flags: CGEventFlags) {
  let modifiers: [(UInt16, CGEventFlags)] = [
    (55, .maskCommand), (56, .maskShift), (58, .maskAlternate), (59, .maskControl),
  ]
  var held: CGEventFlags = []
  for (k, f) in modifiers where flags.contains(f) {
    held.insert(f)
    let e = CGEvent(keyboardEventSource: nil, virtualKey: k, keyDown: true)!
    e.flags = held
    e.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: 0.015)
  }
  for down in [true, false] {
    let e = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down)!
    e.flags = flags
    e.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: 0.035)
  }
  for (k, f) in modifiers.reversed() where flags.contains(f) {
    held.remove(f)
    let e = CGEvent(keyboardEventSource: nil, virtualKey: k, keyDown: false)!
    e.flags = held
    e.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: 0.015)
  }
}
switch args[1] {
case "quit": print(app(args[2]).terminate())
case "state":
  print(
    "AX=\(AXIsProcessTrusted()) capture=\(CGPreflightScreenCaptureAccess()) front=\(NSWorkspace.shared.frontmostApplication?.bundleIdentifier ?? "none")"
  )
  for a in NSWorkspace.shared.runningApplications
  where ["com.clipman.nativeqa", "com.clipman.manager", "com.apple.TextEdit"].contains(
    a.bundleIdentifier ?? "") || a.executableURL?.lastPathComponent == "clipman"
  {
    print("app \(a.bundleIdentifier ?? "unbundled-clipman") pid=\(a.processIdentifier)")
  }
case "tree": tree(AXUIElementCreateApplication(app(args[2]).processIdentifier))
case "focus": print(app(args[2]).activate(options: [.activateAllWindows]))
case "key":
  let expected = args[2]
  let actual = NSWorkspace.shared.frontmostApplication?.bundleIdentifier ?? ""
  guard actual == expected else { fatalError("Wrong foreground: \(actual), expected \(expected)") }
  if expected == "com.apple.TextEdit",
    let name = ProcessInfo.processInfo.environment["CLIPMAN_QA_RECEIVER"]
  {
    let target = AXUIElementCreateApplication(app(expected).processIdentifier)
    let window = attr(target, "AXFocusedWindow")
    guard let window, (attr(window as! AXUIElement, "AXTitle") as? String)?.hasPrefix(name) == true
    else { fatalError("Refusing to type into a non-QA document") }
  }
  var flags: CGEventFlags = []
  for f in args.dropFirst(4) {
    switch f {
    case "cmd": flags.insert(.maskCommand)
    case "shift": flags.insert(.maskShift)
    case "alt": flags.insert(.maskAlternate)
    case "ctrl": flags.insert(.maskControl)
    default: break
    }
  }
  key(UInt16(args[3])!, flags)
case "search-enter":
  guard NSWorkspace.shared.frontmostApplication?.bundleIdentifier == args[2] else {
    fatalError("Wrong foreground")
  }
  let chars = Array(args[3].utf16)
  for down in [true, false] {
    let e = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: down)!
    e.flags = []
    e.keyboardSetUnicodeString(stringLength: chars.count, unicodeString: chars)
    e.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: 0.005)
  }
  key(36, [])
case "backup":
  let payload = (NSPasteboard.general.pasteboardItems ?? []).map { item in
    Dictionary(
      uniqueKeysWithValues: item.types.compactMap { t in
        item.data(forType: t).map { (t.rawValue, $0) }
      })
  }
  try PropertyListSerialization.data(fromPropertyList: payload, format: .binary, options: 0).write(
    to: URL(fileURLWithPath: args[2]))
  try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: args[2])
  print("Clipboard backed up without displaying contents")
case "restore":
  let payload =
    try PropertyListSerialization.propertyList(
      from: Data(contentsOf: URL(fileURLWithPath: args[2])), options: [], format: nil)
    as! [[String: Data]]
  let items = try payload.map { values -> NSPasteboardItem in
    let item = NSPasteboardItem()
    for (type, data) in values {
      guard item.setData(data, forType: NSPasteboard.PasteboardType(type)) else {
        throw NSError(
          domain: "ClipManQA", code: 1,
          userInfo: [
            NSLocalizedDescriptionKey:
              "Clipboard representation could not be restored; backup retained"
          ])
      }
    }
    return item
  }
  NSPasteboard.general.clearContents()
  if !items.isEmpty && !NSPasteboard.general.writeObjects(items) {
    throw NSError(
      domain: "ClipManQA", code: 2,
      userInfo: [NSLocalizedDescriptionKey: "Clipboard restore failed; backup retained"])
  }
  let restored = NSPasteboard.general.pasteboardItems ?? []
  guard
    restored.count == payload.count
      && zip(restored, payload).allSatisfy({ item, values in
        values.allSatisfy { type, data in
          item.data(forType: NSPasteboard.PasteboardType(type)) == data
        }
      })
  else {
    throw NSError(
      domain: "ClipManQA", code: 3,
      userInfo: [NSLocalizedDescriptionKey: "Clipboard restore was incomplete; backup retained"])
  }
  print("Clipboard restored")
case "copy":
  NSPasteboard.general.clearContents()
  NSPasteboard.general.setString(args[2], forType: .string)
  print("Synthetic text copied")
case "copyimage":
  let image = NSImage(size: NSSize(width: 96, height: 64))
  image.lockFocus()
  NSColor(
    calibratedRed: Double.random(in: 0...1), green: Double.random(in: 0...1),
    blue: Double.random(in: 0...1), alpha: 1
  ).setFill()
  NSRect(x: 0, y: 0, width: 96, height: 64).fill()
  NSColor.white.setFill()
  NSRect(x: 24, y: 16, width: 48, height: 32).fill()
  image.unlockFocus()
  NSPasteboard.general.clearContents()
  print(
    NSPasteboard.general.setData(
      NSBitmapImageRep(data: image.tiffRepresentation!)!.representation(
        using: .png, properties: [:])!, forType: .png))
case "read-receiver":
  func text(_ e: AXUIElement) -> String? {
    if (attr(e, "AXRole") as? String) == "AXTextArea" { return attr(e, "AXValue") as? String }
    for c in attr(e, "AXChildren") as? [AXUIElement] ?? [] { if let v = text(c) { return v } }
    return nil
  }
  for w in attr(
    AXUIElementCreateApplication(app("com.apple.TextEdit").processIdentifier), "AXWindows")
    as? [AXUIElement] ?? [] where (attr(w, "AXTitle") as? String)?.hasPrefix(args[2]) == true
  { print(text(w) ?? "NO TEXT AREA", terminator: "") }
case "press":
  func find(_ e: AXUIElement, _ depth: Int = 0) -> Bool {
    if depth > 15 { return false }
    if ["AXTitle", "AXDescription"].contains(where: { (attr(e, $0) as? String) == args[3] }) {
      let result = AXUIElementPerformAction(e, kAXPressAction as CFString)
      if result == .success { return true }
    }
    for c in attr(e, "AXChildren") as? [AXUIElement] ?? [] { if find(c, depth + 1) { return true } }
    return false
  }
  print(find(AXUIElementCreateApplication(app(args[2]).processIdentifier)))
default: fatalError("Unknown command")
}
