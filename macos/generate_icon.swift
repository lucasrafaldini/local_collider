import Cocoa

func renderIcon(size: Int, outputPath: String) {
    let img = NSImage(size: NSSize(width: size, height: size))
    img.lockFocus()
    guard let ctx = NSGraphicsContext.current?.cgContext else { return }
    let s = CGFloat(size)

    // Rounded macOS squircle background
    let rect = CGRect(x: s * 0.06, y: s * 0.06, width: s * 0.88, height: s * 0.88)
    let path = NSBezierPath(roundedRect: rect, xRadius: s * 0.20, yRadius: s * 0.20)
    NSColor(calibratedRed: 0.05, green: 0.07, blue: 0.12, alpha: 1.0).setFill()
    path.fill()

    // Outer synchrotron glowing ring
    ctx.setStrokeColor(CGColor(red: 0.0, green: 0.85, blue: 0.95, alpha: 0.95))
    ctx.setLineWidth(s * 0.055)
    ctx.strokeEllipse(in: CGRect(x: s * 0.18, y: s * 0.18, width: s * 0.64, height: s * 0.64))

    // Inner quantum resonance ring
    ctx.setStrokeColor(CGColor(red: 0.88, green: 0.25, blue: 0.95, alpha: 0.85))
    ctx.setLineWidth(s * 0.025)
    ctx.strokeEllipse(in: CGRect(x: s * 0.30, y: s * 0.30, width: s * 0.40, height: s * 0.40))

    // Particle collision tracks from center
    let center = CGPoint(x: s * 0.5, y: s * 0.5)
    let angles: [CGFloat] = [0.3, 1.1, 2.2, 3.5, 4.4, 5.5]
    for (idx, a) in angles.enumerated() {
        ctx.setStrokeColor(
            idx % 2 == 0
                ? CGColor(red: 1.0, green: 0.85, blue: 0.2, alpha: 0.95)
                : CGColor(red: 0.3, green: 1.0, blue: 0.55, alpha: 0.95)
        )
        ctx.setLineWidth(s * 0.022)
        ctx.move(to: center)
        let r = s * 0.30
        ctx.addLine(to: CGPoint(x: center.x + r * cos(a), y: center.y + r * sin(a)))
        ctx.strokePath()
    }

    // Bright interaction vertex core
    ctx.setFillColor(CGColor(red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0))
    ctx.fillEllipse(in: CGRect(x: s * 0.44, y: s * 0.44, width: s * 0.12, height: s * 0.12))

    img.unlockFocus()

    if let tiff = img.tiffRepresentation,
       let rep = NSBitmapImageRep(data: tiff),
       let png = rep.representation(using: .png, properties: [:]) {
        try? png.write(to: URL(fileURLWithPath: outputPath))
    }
}

let args = CommandLine.arguments
if args.count > 1 {
    renderIcon(size: 512, outputPath: args[1])
}
