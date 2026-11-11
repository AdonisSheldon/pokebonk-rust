import AppKit

let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
func color(_ hex: UInt32) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255,
            green: CGFloat((hex >> 8) & 255) / 255,
            blue: CGFloat(hex & 255) / 255, alpha: 1)
}
func ball(_ x: CGFloat, _ y: CGFloat, _ size: CGFloat) {
    let circle = NSBezierPath(ovalIn: NSRect(x: x, y: y, width: size, height: size))
    color(0xFFFCF5).setFill(); circle.fill()
    NSGraphicsContext.saveGraphicsState(); circle.addClip()
    color(0xEA5B60).setFill(); NSRect(x: x, y: y, width: size, height: size / 2).fill()
    NSGraphicsContext.restoreGraphicsState()
    color(0x303048).setStroke(); circle.lineWidth = max(2, size / 24); circle.stroke()
    let seam = NSBezierPath()
    seam.move(to: NSPoint(x: x, y: y + size / 2))
    seam.line(to: NSPoint(x: x + size, y: y + size / 2))
    seam.lineWidth = circle.lineWidth; seam.stroke()
    let hub = NSBezierPath(ovalIn: NSRect(x: x + size * 0.35, y: y + size * 0.35,
                                      width: size * 0.3, height: size * 0.3))
    color(0xFFFCF5).setFill(); hub.fill(); hub.lineWidth = circle.lineWidth; hub.stroke()
}
for (name, width, height) in [("windows-header", 150, 57), ("windows-sidebar", 164, 314)] {
    let art = NSImage(size: NSSize(width: width, height: height), flipped: true) { _ in
        color(0xF5F1E8).setFill()
        NSRect(x: 0, y: 0, width: width, height: height).fill()
        if name == "windows-header" {
            ball(100, 9, 34)
            color(0xEA5B60).setFill()
            for x in [58, 70, 82] { NSRect(x: x, y: 24, width: 4, height: 4).fill() }
        } else {
            let frame = NSBezierPath()
            frame.move(to: NSPoint(x: 28, y: 40))
            for point in [NSPoint(x: 136, y: 40), NSPoint(x: 136, y: 48),
                          NSPoint(x: 144, y: 48), NSPoint(x: 144, y: 266),
                          NSPoint(x: 136, y: 266), NSPoint(x: 136, y: 274),
                          NSPoint(x: 28, y: 274), NSPoint(x: 28, y: 266),
                          NSPoint(x: 20, y: 266), NSPoint(x: 20, y: 48),
                          NSPoint(x: 28, y: 48)] { frame.line(to: point) }
            frame.close(); color(0xFFFCF5).setFill(); frame.fill()
            color(0x303048).setStroke(); frame.lineWidth = 2; frame.stroke()
            ball(44, 119, 76)
        }
        return true
    }
    let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)!
    art.draw(in: NSRect(x: 0, y: 0, width: width, height: height))
    NSGraphicsContext.restoreGraphicsState()
    // NSIS needs a Windows BITMAPINFOHEADER and uncompressed 24-bit BGR pixels.
    var bitmap = Data([0x42, 0x4D])
    func append<T: FixedWidthInteger>(_ value: T) {
        var little = value.littleEndian
        withUnsafeBytes(of: &little) { bitmap.append(contentsOf: $0) }
    }
    let rowSize = (width * 3 + 3) / 4 * 4
    append(UInt32(54 + rowSize * height)); append(UInt32(0)); append(UInt32(54))
    append(UInt32(40)); append(Int32(width)); append(Int32(height))
    append(UInt16(1)); append(UInt16(24)); append(UInt32(0)); append(UInt32(rowSize * height))
    append(Int32(2835)); append(Int32(2835)); append(UInt32(0)); append(UInt32(0))
    precondition(rep.bitsPerPixel == 32 && !rep.bitmapFormat.contains(.alphaFirst))
    let pixels = rep.bitmapData!
    for y in (0..<height).reversed() {
        for x in 0..<width {
            let index = y * rep.bytesPerRow + x * 4
            bitmap.append(pixels[index + 2]); bitmap.append(pixels[index + 1]); bitmap.append(pixels[index])
        }
        bitmap.append(contentsOf: repeatElement(UInt8(0), count: rowSize - width * 3))
    }
    precondition(bitmap.count == 54 + rowSize * height)
    try bitmap.write(to: output.appendingPathComponent(name + ".bmp"))
}
print("Rendered text-free NSIS artwork: 150×57 header and 164×314 sidebar")
