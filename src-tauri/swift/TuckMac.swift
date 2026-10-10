import AppKit
import Foundation
import QuickLookUI

// Called on the app's main thread. Reuse one native preview window.
private final class PreviewPanel: NSPanel {
    override func cancelOperation(_ sender: Any?) { close() }
}
private final class PreviewLifecycle: NSObject, NSWindowDelegate {
    func windowWillClose(_ notification: Notification) { previewView?.previewItem = nil }
}
private let previewLifecycle = PreviewLifecycle()
private var previewWindow: NSPanel?
private var previewView: QLPreviewView?
@_cdecl("tuck_mac_preview")
public func tuckMacPreview(_ path: UnsafePointer<CChar>) -> Int32 {
    guard Thread.isMainThread else { return 0 }
    let url = URL(fileURLWithPath: String(cString: path))
    if previewWindow == nil {
        let panel = PreviewPanel(contentRect: NSRect(x: 0, y: 0, width: 720, height: 820),
                            styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
        panel.isReleasedWhenClosed = false
        panel.delegate = previewLifecycle
        panel.minSize = NSSize(width: 360, height: 400)
        guard let view = QLPreviewView(frame: panel.contentView!.bounds, style: .normal) else { return 0 }
        view.autoresizingMask = [.width, .height]
        view.autostarts = true
        panel.contentView = view
        panel.center()
        previewWindow = panel
        previewView = view
    }
    previewView?.previewItem = url as NSURL
    previewWindow?.title = url.lastPathComponent
    previewWindow?.makeKeyAndOrderFront(nil)
    return 1
}

private struct FolderBookmark: Codable {
    let originalPath: String
    let data: Data
}
private let accessLock = NSLock()
private var accessURL: URL?
private func retainAccess(_ url: URL) {
    if url == accessURL { return }
    let started = url.startAccessingSecurityScopedResource()
    accessURL?.stopAccessingSecurityScopedResource()
    accessURL = started ? url : nil
}
private func reply(_ object: [String: String]) -> UnsafeMutablePointer<CChar>? {
    guard let bytes = try? JSONSerialization.data(withJSONObject: object),
          let string = String(data: bytes, encoding: .utf8) else { return nil }
    return strdup(string)
}
// The grant stays in the Tauri process. A subprocess cannot grant its parent sandbox access.
@_cdecl("tuck_mac_folder_access")
public func tuckMacFolderAccess(_ root: UnsafePointer<CChar>, _ file: UnsafePointer<CChar>, _ restore: Int32) -> UnsafeMutablePointer<CChar>? {
    accessLock.lock(); defer { accessLock.unlock() }
    let path = String(cString: root)
    let storage = URL(fileURLWithPath: String(cString: file))
    do {
        if restore != 0 {
            guard FileManager.default.fileExists(atPath: storage.path) else { return reply(["path":path]) }
            let saved = try JSONDecoder().decode(FolderBookmark.self, from: Data(contentsOf: storage))
            guard saved.originalPath == path else { return reply(["path":path]) }
            var stale = false
            let url = try URL(resolvingBookmarkData: saved.data, options: [.withSecurityScope, .withoutUI],
                              relativeTo: nil, bookmarkDataIsStale: &stale)
            retainAccess(url)
            if stale {
                let data = try url.bookmarkData(options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil)
                try JSONEncoder().encode(FolderBookmark(originalPath: path, data: data)).write(to: storage, options: .atomic)
            }
            return reply(["path":url.path])
        }
        let url = URL(fileURLWithPath: path, isDirectory: true)
        let data = try url.bookmarkData(options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil)
        try JSONEncoder().encode(FolderBookmark(originalPath: path, data: data)).write(to: storage, options: .atomic)
        var stale = false
        let resolved = try URL(resolvingBookmarkData: data, options: [.withSecurityScope, .withoutUI], relativeTo: nil, bookmarkDataIsStale: &stale)
        retainAccess(resolved)
        return reply(["path":resolved.path])
    } catch {
        return reply(["error":"Folder access could not be restored or saved. Choose the folder again."])
    }
}
@_cdecl("tuck_mac_free")
public func tuckMacFree(_ value: UnsafeMutablePointer<CChar>?) { free(value) }
