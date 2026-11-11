import AppKit

let width = 640, height = 360
let output = CommandLine.arguments[1]
func color(_ hex: UInt32) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255,
            green: CGFloat((hex >> 8) & 255) / 255,
            blue: CGFloat(hex & 255) / 255, alpha: 1)
}
func pixelShape(_ points: [NSPoint], fill: UInt32, border: UInt32? = nil) {
    let path = NSBezierPath()
    path.move(to: points[0])
    for point in points.dropFirst() { path.line(to: point) }
    path.close()
    color(fill).setFill(); path.fill()
    if let border = border { color(border).setStroke(); path.lineWidth = 2; path.stroke() }
}
let art = NSImage(size: NSSize(width: width, height: height), flipped: true) { _ in
    color(0xF5F1E8).setFill()
    NSRect(x: 0, y: 0, width: width, height: height).fill()
    for x: CGFloat in [84, 404] {
        pixelShape([NSPoint(x: x + 8, y: 75), NSPoint(x: x + 144, y: 75),
                    NSPoint(x: x + 144, y: 83), NSPoint(x: x + 152, y: 83),
                    NSPoint(x: x + 152, y: 245), NSPoint(x: x + 144, y: 245),
                    NSPoint(x: x + 144, y: 253), NSPoint(x: x + 8, y: 253),
                    NSPoint(x: x + 8, y: 245), NSPoint(x: x, y: 245),
                    NSPoint(x: x, y: 83), NSPoint(x: x + 8, y: 83)],
                   fill: 0xFFFCF5, border: 0x303048)
    }
    pixelShape([NSPoint(x: 292, y: 139), NSPoint(x: 324, y: 139),
                NSPoint(x: 324, y: 123), NSPoint(x: 350, y: 145),
                NSPoint(x: 324, y: 167), NSPoint(x: 324, y: 151),
                NSPoint(x: 292, y: 151)], fill: 0xEA5B60)
    let ball = NSBezierPath(ovalIn: NSRect(x: 308, y: 35, width: 24, height: 24))
    color(0xFFFCF5).setFill(); ball.fill()
    NSGraphicsContext.saveGraphicsState(); ball.addClip()
    color(0xEA5B60).setFill(); NSRect(x: 308, y: 35, width: 24, height: 12).fill()
    NSGraphicsContext.restoreGraphicsState()
    color(0x303048).setStroke(); ball.lineWidth = 2; ball.stroke()
    let seam = NSBezierPath()
    seam.move(to: NSPoint(x: 308, y: 47)); seam.line(to: NSPoint(x: 332, y: 47))
    seam.lineWidth = 2; seam.stroke()
    let hub = NSBezierPath(ovalIn: NSRect(x: 316, y: 43, width: 8, height: 8))
    color(0xFFFCF5).setFill(); hub.fill(); hub.lineWidth = 2; hub.stroke()
    return true
}

let combined = NSImage(size: NSSize(width: width, height: height))
for scale in [1, 2] {
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width * scale,
        pixelsHigh: height * scale, bitsPerSample: 8, samplesPerPixel: 4,
        hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB,
        bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
    art.draw(in: NSRect(x: 0, y: 0, width: width * scale, height: height * scale))
    NSGraphicsContext.restoreGraphicsState()
    rep.size = NSSize(width: width, height: height)
    combined.addRepresentation(rep)
    if scale == 1 {
        try rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: output + ".png"))
    }
}
try combined.tiffRepresentation(using: .lzw, factor: 1)!.write(to: URL(fileURLWithPath: output + ".tiff"))
print("Rendered DMG background at 640×360 points with 1× and 2× representations")
