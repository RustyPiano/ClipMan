#!/usr/bin/env python3
"""Native smoke test of an already-built com.clipman.nativeqa app (macOS only).
Usage: python3 scripts/native-qa/run.py '/absolute/path/ClipMan QA.app'
Requires Accessibility permission for the terminal/agent and QA app. See AGENTS.md.
"""
import json
import os
from pathlib import Path
import plistlib
import sqlite3
import subprocess
import sys
import tempfile
import time

QA = 'com.clipman.nativeqa'
TARGET = 'com.apple.TextEdit'


def main():
    if sys.platform != 'darwin': raise RuntimeError('This test requires macOS')
    if len(sys.argv) < 2: raise SystemExit(__doc__)
    storage_only = '--storage-only' in sys.argv
    bundle = Path(sys.argv[1]).resolve()
    info = plistlib.loads((bundle / 'Contents/Info.plist').read_bytes())
    if info.get('CFBundleIdentifier') != QA: raise RuntimeError('Refusing to use the daily ClipMan database')
    work = Path(tempfile.mkdtemp(prefix='clipman-native-qa-'))
    control = work / 'control'
    subprocess.run(['swiftc', str(Path(__file__).with_name('control.swift')), '-o', str(control)], check=True)

    def ctl(*args):
        return subprocess.check_output([str(control), *map(str, args)], text=True)

    def wait(check, message):
        until = time.monotonic() + 5
        while time.monotonic() < until:
            if check():
                return
            time.sleep(.05)
        raise AssertionError(message)

    def front(app):
        return 'front=' + app + '\n' in ctl('state')

    state = ctl('state')
    if 'AX=true' not in state: raise RuntimeError('Grant the terminal/agent Accessibility permission first')
    if any(line.startswith('app ') and not line.startswith(f'app {TARGET} ') for line in state.splitlines()):
        raise RuntimeError('Quit every ClipMan instance before running native QA')
    app_data = Path.home() / 'Library/Application Support' / QA
    settings_file = app_data / 'settings.json'
    database = app_data / 'clipman.db'
    if app_data.is_symlink(): raise RuntimeError('QA data directory must not be a symlink')
    if settings_file.exists():
        saved = json.loads(settings_file.read_text())
        configured = saved.get('settings', saved)
        if configured.get('customDataPath') or configured.get('custom_data_path'):
            raise RuntimeError('QA must use its default data directory; customDataPath is not allowed')
    receiver = work / f'receiver-{work.name}.txt'
    os.environ['CLIPMAN_QA_RECEIVER'] = receiver.name
    receiver.write_text('ClipMan native QA target\n')
    backup = work / 'clipboard.plist'
    ctl('backup', backup)
    print('Test evidence:', work, flush=True)
    results = []
    started = False
    try:
        # Replace the pasteboard before starting the isolated monitor.
        ctl('copy', 'CLIPMAN_QA_START_' + work.name)
        if not storage_only:
            subprocess.run(['open', '-a', 'TextEdit', str(receiver)], check=True)
        subprocess.run(['open', str(bundle)], check=True)
        started = True
        wait(lambda: f'app {QA} ' in ctl('state'), 'QA app did not start')
        wait(database.exists, 'QA database was not created')
        wait(lambda: any(label in ctl('tree', QA).splitlines() for label in ['Pause Capture', '暂停采集']),
             'Native tray menu did not initialize')
        settings = json.loads(settings_file.read_text())['settings'] if settings_file.exists() else {}
        pause_label = 'Pause Capture' if 'Pause Capture' in ctl('tree', QA).splitlines() else '暂停采集'
        if not storage_only and not settings.get('autoPaste', True): raise RuntimeError('QA settings must enable auto-paste')
        if not storage_only and settings.get('globalShortcut', 'CommandOrControl+Shift+V') != 'CommandOrControl+Shift+V': raise RuntimeError('QA shortcut must use the default')
        if settings.get('capturePaused'):
            if ctl('press', QA, pause_label).strip() != 'true': raise RuntimeError('Pause menu item not found')
            wait(lambda: not json.loads(settings_file.read_text())['settings']['capturePaused'], 'Capture did not resume')
        if not storage_only:
            ctl('focus', TARGET)
            wait(lambda: front(TARGET), 'TextEdit did not become foreground')

        def captured(value):
            with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as conn:
                return conn.execute('SELECT count(*) FROM clips WHERE content=?', (value.encode(),)).fetchone()[0] > 0

        for index in range(0 if storage_only else 8):
            value = f'CLIPMAN_QA_{work.name}_MATCH_{index}_中文'
            ctl('copy', value)
            wait(lambda: captured(value), 'Synthetic clipboard text was not captured')
            if index % 2:
                distractor = f'CLIPMAN_QA_{work.name}_DISTRACTOR_{index}'
                ctl('copy', distractor)
                wait(lambda: captured(distractor), 'Distractor was not captured')
            ctl('focus', TARGET)
            wait(lambda: front(TARGET), 'Wrong target foreground')
            ctl('key', TARGET, 0, 'cmd')
            ctl('key', TARGET, 9, 'cmd', 'shift')
            wait(lambda: front(QA), 'Global shortcut did not open QuickBar')
            if index % 2:
                ctl('search-enter', QA, f'{work.name}_MATCH_{index}')
            else:
                ctl('key', QA, 36)
            wait(lambda: ctl('read-receiver', receiver.name) == value,
                 'TextEdit content mismatch; verify QA Accessibility permission and visible dialogs')
            wait(lambda: front(TARGET), 'Paste did not restore TextEdit focus')
            results.append({'iteration': index, 'search': bool(index % 2), 'passed': True})
            print(results[-1], flush=True)

        # Small image must preserve pixels and reuse PNG bytes for its thumbnail.
        ctl('copyimage')
        def small_image():
            with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as conn:
                return conn.execute("SELECT content = thumbnail FROM clips WHERE content_type='image' ORDER BY timestamp DESC LIMIT 1").fetchone() == (1,)
        wait(small_image, 'Small image thumbnail differs from original PNG')
        results.append({'smallImageReusesOriginal': True})
        # Force a read failure only in the isolated QA database, then restore its schema.
        before = {line for line in ctl('tree', QA).splitlines() if 'CLIPMAN_QA_' in line}
        if not before:
            raise RuntimeError('QA entries were not visible in the native menu tree')
        with sqlite3.connect(database) as conn:
            conn.execute('ALTER TABLE clips RENAME TO qa_hidden_clips')
        try:
            if ctl('press', QA, pause_label).strip() != 'true':
                raise RuntimeError('Pause menu item was not available')
            wait(lambda: json.loads(settings_file.read_text())['settings']['capturePaused'], 'Pause did not complete')
            time.sleep(.2)
            after = set(ctl('tree', QA).splitlines())
            if not before <= after:
                raise RuntimeError('Failed DB read replaced the previous tray entries')
        finally:
            with sqlite3.connect(database) as conn:
                conn.execute('ALTER TABLE qa_hidden_clips RENAME TO clips')
        results.append({'trayReadFailurePreservesMenu': True})
        print('PASS:', results, flush=True)
    except Exception as error:
        (work / 'failure.txt').write_text(str(error) + '\n' + ctl('state'))
        raise
    finally:
        (work / 'results.json').write_text(json.dumps(results, ensure_ascii=False, indent=2))
        # Stop the monitor before restoring private clipboard content, even after failure.
        if started and f'app {QA} ' in ctl('state'):
            ctl('quit', QA)
            wait(lambda: f'app {QA} ' not in ctl('state'), 'QA app still running; clipboard backup retained')
        ctl('restore', backup)
        backup.unlink()
        if not storage_only and front(TARGET):
            ctl('key', TARGET, 1, 'cmd')  # Save the synthetic receiver for inspection.
            ctl('key', TARGET, 13, 'cmd')
        print('Original clipboard restored; QA app stopped. Evidence:', work, flush=True)


if __name__ == '__main__':
    main()
