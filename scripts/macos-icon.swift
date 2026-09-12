// Source artwork for 3D Canon. Render with AppKit; no external assets or packages.
import AppKit

guard CommandLine.arguments.count == 2 else {
    fatalError("Usage: swift scripts/macos-icon.swift OUTPUT.iconset")
}
let output = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true)
try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)

func color(_ hex: UInt32) -> NSColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 255) / 255,
            green: CGFloat((hex >> 8) & 255) / 255,
            blue: CGFloat(hex & 255) / 255, alpha: 1)
}

func rounded(_ rect: NSRect, _ radius: CGFloat, _ fill: UInt32) {
    color(fill).setFill()
    NSBezierPath(roundedRect: rect, xRadius: radius, yRadius: radius).fill()
}

func polygon(_ points: [NSPoint], _ fill: UInt32) {
    let path = NSBezierPath()
    path.move(to: points[0])
    for point in points.dropFirst() { path.line(to: point) }
    path.close()
    color(fill).setFill()
    path.fill()
}

func ellipse(_ rect: NSRect, _ fill: UInt32) {
    color(fill).setFill()
    NSBezierPath(ovalIn: rect).fill()
}

func artwork() {
    let tile = NSBezierPath(roundedRect: NSRect(x: 32, y: 32, width: 960, height: 960),
                            xRadius: 212, yRadius: 212)
    NSGraphicsContext.saveGraphicsState()
    let shadow = NSShadow()
    shadow.shadowColor = NSColor.black.withAlphaComponent(0.25)
    shadow.shadowBlurRadius = 18
    shadow.shadowOffset = NSSize(width: 0, height: -8)
    shadow.set()
    color(0x132B3A).setFill()
    tile.fill()
    NSGraphicsContext.restoreGraphicsState()

    NSGraphicsContext.saveGraphicsState()
    tile.addClip()
    NSGradient(starting: color(0x112332), ending: color(0x28667C))!
        .draw(in: tile, angle: 70)

    // Distant blue hills and a faceted grass island.
    polygon([NSPoint(x: 0, y: 410), NSPoint(x: 185, y: 505), NSPoint(x: 410, y: 445),
             NSPoint(x: 640, y: 585), NSPoint(x: 1030, y: 460), NSPoint(x: 1030, y: 0),
             NSPoint(x: 0, y: 0)], 0x326B73)
    polygon([NSPoint(x: 65, y: 250), NSPoint(x: 318, y: 405), NSPoint(x: 867, y: 325),
             NSPoint(x: 931, y: 205), NSPoint(x: 582, y: 92)], 0x294239)
    polygon([NSPoint(x: 65, y: 315), NSPoint(x: 325, y: 459), NSPoint(x: 870, y: 378),
             NSPoint(x: 931, y: 270), NSPoint(x: 582, y: 160)], 0x8AAA58)
    polygon([NSPoint(x: 65, y: 315), NSPoint(x: 325, y: 459), NSPoint(x: 582, y: 160)], 0x668F51)
    polygon([NSPoint(x: 325, y: 459), NSPoint(x: 870, y: 378), NSPoint(x: 582, y: 160)], 0xA9BC69)

    // A dotted ballistic arc ending in a bright shell.
    for i in 0..<7 {
        let t = CGFloat(i) / 6
        let x = 556 + t * 248
        let y = 743 + 89 * sin(t * .pi * 0.72)
        let radius: CGFloat = 7 + 3 * t
        ellipse(NSRect(x: x - radius, y: y - radius, width: radius * 2, height: radius * 2), 0xFFD16B)
    }
    ellipse(NSRect(x: 833, y: 741, width: 58, height: 58), 0xFFDC81)
    ellipse(NSRect(x: 845, y: 762, width: 15, height: 15), 0xFFF5D6)

    // Cannon carriage: warm red face, darker side, black iron barrel and brass muzzle.
    rounded(NSRect(x: 248, y: 338, width: 359, height: 105), 24, 0xB54339)
    polygon([NSPoint(x: 253, y: 425), NSPoint(x: 328, y: 518), NSPoint(x: 552, y: 508),
             NSPoint(x: 621, y: 413)], 0xF66D52)
    polygon([NSPoint(x: 552, y: 508), NSPoint(x: 621, y: 413), NSPoint(x: 601, y: 337),
             NSPoint(x: 541, y: 414)], 0xD14E40)

    NSGraphicsContext.saveGraphicsState()
    let barrelTransform = NSAffineTransform()
    barrelTransform.translateX(by: 400, yBy: 458)
    barrelTransform.rotate(byDegrees: 43)
    barrelTransform.concat()
    rounded(NSRect(x: -52, y: -64, width: 374, height: 128), 34, 0x10232D)
    rounded(NSRect(x: -23, y: 10, width: 324, height: 34), 15, 0x6B8C98)
    rounded(NSRect(x: 271, y: -75, width: 64, height: 150), 12, 0xD7A344)
    rounded(NSRect(x: 281, y: 23, width: 44, height: 43), 7, 0xFFDA80)
    rounded(NSRect(x: -44, y: -65, width: 45, height: 130), 15, 0x243D48)
    NSGraphicsContext.restoreGraphicsState()

    for x in [CGFloat(296), CGFloat(510)] {
        ellipse(NSRect(x: x - 88, y: 250, width: 176, height: 176), 0x122832)
        ellipse(NSRect(x: x - 64, y: 274, width: 128, height: 128), 0x304853)
        ellipse(NSRect(x: x - 35, y: 303, width: 70, height: 70), 0xF1BE5E)
        ellipse(NSRect(x: x - 13, y: 325, width: 26, height: 26), 0x946A31)
    }
    NSGraphicsContext.restoreGraphicsState()

    NSColor.white.withAlphaComponent(0.12).setStroke()
    tile.lineWidth = 3
    tile.stroke()
}

for size in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let pixels = size * scale
        let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
                                      bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true,
                                      isPlanar: false, colorSpaceName: .deviceRGB,
                                      bytesPerRow: 0, bitsPerPixel: 0)!
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
        NSGraphicsContext.current!.imageInterpolation = .high
        let transform = NSAffineTransform()
        transform.scale(by: CGFloat(pixels) / 1024)
        transform.concat()
        artwork()
        NSGraphicsContext.restoreGraphicsState()
        let suffix = scale == 2 ? "@2x" : ""
        let file = output.appendingPathComponent("icon_\(size)x\(size)\(suffix).png")
        try bitmap.representation(using: .png, properties: [:])!.write(to: file)
    }
}
print("Rendered 3D Canon icon at 16–1024 pixels")
