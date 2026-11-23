import Cocoa
import Foundation

struct DiscoveryItem: Decodable {
    let id: String
    let name: String
    let symbol: String
}

struct TelemetrySnapshot: Decodable {
    let uptime_secs: UInt64
    let active_threads: Int
    let max_threads: Int
    let intensity_pct: UInt8
    let system_cpu_pct: Float
    let memory_used_mb: UInt64
    let hashes_per_sec: Double
    let gflops: Double
    let beam_energy_tev: Double
    let peak_energy_tev: Double
    let lorentz_gamma: Double
    let nines_of_c: Double
    let collisions_per_sec: Double
    let quantum_proximity_pct: Double
    let next_target_name: String
    let next_target_symbol: String
    let next_target_sigma_progress: Double
    let discoveries: [DiscoveryItem]
}

class MenuBarDelegate: NSObject, NSApplicationDelegate {
    var statusItem: NSStatusItem!
    var timer: Timer?
    var host: String = "127.0.0.1"
    var port: UInt16 = 7878
    var statusOnlyOnLaunch: Bool = false

    // Menu items updated dynamically every second
    var stateMenuItem: NSMenuItem!
    var cpuMenuItem: NSMenuItem!
    var beamMenuItem: NSMenuItem!
    var quantumMenuItem: NSMenuItem!

    var binDir: String = ""
    var workDir: String = ""

    func applicationDidFinishLaunching(_ notification: Notification) {
        parseArguments()
        resolveDirectories()

        // Prevent duplicate menu bar icons if another instance is already running
        if isAnotherInstanceRunning() {
            if !statusOnlyOnLaunch {
                openAllThreeTerminals()
            }
            NSApplication.shared.terminate(nil)
            return
        }

        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        if let button = statusItem.button {
            button.font = NSFont.monospacedDigitSystemFont(ofSize: 12.5, weight: .semibold)
            button.title = "⚛ OFF · 0% CPU"
            button.toolTip = "Local Collider — Particle Accelerator & CPU Monitor"
        }

        buildMenu()
        refreshTelemetry()

        timer = Timer.scheduledTimer(
            timeInterval: 1.0,
            target: self,
            selector: #selector(refreshTelemetry),
            userInfo: nil,
            repeats: true
        )
        if let t = timer {
            RunLoop.main.add(t, forMode: .common)
        }

        // If double-clicked as Local Collider.app (not spawned with --status-only), open all 3 terminals!
        if !statusOnlyOnLaunch {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) {
                self.openAllThreeTerminals()
            }
        }
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        openAllThreeTerminals()
        return true
    }

    func isAnotherInstanceRunning() -> Bool {
        let myPid = ProcessInfo.processInfo.processIdentifier
        let apps = NSWorkspace.shared.runningApplications.filter {
            $0.executableURL?.lastPathComponent == "LocalColliderMenuBar" && $0.processIdentifier != myPid
        }
        return !apps.isEmpty
    }

    func parseArguments() {
        let args = CommandLine.arguments
        var i = 1
        while i < args.count {
            switch args[i] {
            case "--host":
                if i + 1 < args.count { host = args[i + 1]; i += 1 }
            case "--port":
                if i + 1 < args.count, let p = UInt16(args[i + 1]) { port = p; i += 1 }
            case "--status-only":
                statusOnlyOnLaunch = true
            default:
                break
            }
            i += 1
        }
    }

    func resolveDirectories() {
        let execURL = URL(fileURLWithPath: CommandLine.arguments[0]).resolvingSymlinksInPath()
        let execDir = execURL.deletingLastPathComponent().path
        let fm = FileManager.default

        // Check where collider-engine binary resides
        let candidateDirs = [
            execDir,
            "\(fm.currentDirectoryPath)/target/release",
            "\(fm.currentDirectoryPath)/target/debug",
            "/Volumes/VAULT/Code/local_collider/target/release",
            "/Volumes/VAULT/Code/local_collider/target/debug",
        ]

        for dir in candidateDirs {
            if fm.isExecutableFile(atPath: "\(dir)/collider-engine") {
                binDir = dir
                break
            }
        }
        if binDir.isEmpty {
            binDir = execDir
        }

        // Determine working directory for collider_state.json
        if fm.fileExists(atPath: "/Volumes/VAULT/Code/local_collider/Cargo.toml") {
            workDir = "/Volumes/VAULT/Code/local_collider"
        } else {
            let appSupport = fm.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
                .appendingPathComponent("LocalCollider").path
            try? fm.createDirectory(atPath: appSupport, withIntermediateDirectories: true)
            workDir = appSupport
        }
    }

    func buildMenu() {
        let menu = NSMenu()
        menu.autoenablesItems = false

        let headerItem = NSMenuItem(title: "⚛ LOCAL COLLIDER — CPU & BEAM GUARDIAN", action: nil, keyEquivalent: "")
        headerItem.isEnabled = false
        menu.addItem(headerItem)

        stateMenuItem = NSMenuItem(title: "○ Engine Status: STOPPED (0% CPU)", action: nil, keyEquivalent: "")
        stateMenuItem.isEnabled = false
        menu.addItem(stateMenuItem)

        cpuMenuItem = NSMenuItem(title: "CPU Load: 0.0% | 0.0 kH/s | 0.00 GFLOPS", action: nil, keyEquivalent: "")
        cpuMenuItem.isEnabled = false
        menu.addItem(cpuMenuItem)

        beamMenuItem = NSMenuItem(title: "Beam Energy: 0.000 TeV | Lorentz γ = 1.0", action: nil, keyEquivalent: "")
        beamMenuItem.isEnabled = false
        menu.addItem(beamMenuItem)

        quantumMenuItem = NSMenuItem(title: "Quantum 5σ Proximity: 0.0% | Discovered: 0/12", action: nil, keyEquivalent: "")
        quantumMenuItem.isEnabled = false
        menu.addItem(quantumMenuItem)

        menu.addItem(NSMenuItem.separator())

        let openAll = NSMenuItem(
            title: "🚀 Open All 3 Terminals (Engine + Animation + CLI)",
            action: #selector(openAllThreeTerminals),
            keyEquivalent: "o"
        )
        openAll.target = self
        menu.addItem(openAll)

        let openT1 = NSMenuItem(
            title: "1️⃣ Open Terminal 1: Clickable Engine Control Deck",
            action: #selector(openTerminal1Engine),
            keyEquivalent: "1"
        )
        openT1.target = self
        menu.addItem(openT1)

        let openT2 = NSMenuItem(
            title: "2️⃣ Open Terminal 2: TUI Collider Animation (collider-viz)",
            action: #selector(openTerminal2Viz),
            keyEquivalent: "2"
        )
        openT2.target = self
        menu.addItem(openT2)

        let openT3 = NSMenuItem(
            title: "3️⃣ Open Terminal 3: CLI Telemetry & Quantum Codex",
            action: #selector(openTerminal3Cli),
            keyEquivalent: "3"
        )
        openT3.target = self
        menu.addItem(openT3)

        menu.addItem(NSMenuItem.separator())

        let burstItem = NSMenuItem(
            title: "⚡ Fire Supercritical Pulse Burst (3.25x Overdrive)",
            action: #selector(fireSupercriticalBurst),
            keyEquivalent: "b"
        )
        burstItem.target = self
        menu.addItem(burstItem)

        let lockRfItem = NSMenuItem(
            title: "🎯 Auto-Lock RF Harmonic Resonance",
            action: #selector(autoLockRfResonance),
            keyEquivalent: "a"
        )
        lockRfItem.target = self
        menu.addItem(lockRfItem)

        let ecoItem = NSMenuItem(
            title: "🌱 Eco Mode: Throttle to 1 Thread @ 15% CPU",
            action: #selector(setEcoCpuMode),
            keyEquivalent: "e"
        )
        ecoItem.target = self
        menu.addItem(ecoItem)

        let balancedItem = NSMenuItem(
            title: "⚖️ Balanced Mode: 2 Threads @ 65% CPU",
            action: #selector(setBalancedCpuMode),
            keyEquivalent: ""
        )
        balancedItem.target = self
        menu.addItem(balancedItem)

        let maxItem = NSMenuItem(
            title: "🔥 Overdrive Mode: 6 Threads @ 95% CPU",
            action: #selector(setOverdriveCpuMode),
            keyEquivalent: ""
        )
        maxItem.target = self
        menu.addItem(maxItem)

        menu.addItem(NSMenuItem.separator())

        let stopAllItem = NSMenuItem(
            title: "🛑 Stop All Acceleration & Close Terminals (0% CPU)",
            action: #selector(stopAllAccelerationAndCloseTerminals),
            keyEquivalent: "s"
        )
        stopAllItem.target = self
        menu.addItem(stopAllItem)

        let quitItem = NSMenuItem(
            title: "✕ Quit Local Collider & Stop Engine Completely",
            action: #selector(quitAndStopEverything),
            keyEquivalent: "q"
        )
        quitItem.target = self
        menu.addItem(quitItem)

        statusItem.menu = menu
    }

    func sendIPCCommand(_ jsonLine: String) -> TelemetrySnapshot? {
        var inputStream: InputStream?
        var outputStream: OutputStream?
        Stream.getStreamsToHost(withName: host, port: Int(port), inputStream: &inputStream, outputStream: &outputStream)

        guard let inStream = inputStream, let outStream = outputStream else {
            return nil
        }

        inStream.open()
        outStream.open()
        defer {
            inStream.close()
            outStream.close()
        }

        let payload = "\(jsonLine)\n"
        guard let data = payload.data(using: .utf8) else { return nil }
        let written = data.withUnsafeBytes { ptr -> Int in
            guard let base = ptr.baseAddress?.assumingMemoryBound(to: UInt8.self) else { return -1 }
            return outStream.write(base, maxLength: data.count)
        }
        if written <= 0 { return nil }

        var buffer = [UInt8](repeating: 0, count: 65536)
        var accumulated = Data()
        let deadline = Date().addingTimeInterval(0.45)

        while Date() < deadline {
            if inStream.hasBytesAvailable {
                let readCount = inStream.read(&buffer, maxLength: buffer.count)
                if readCount > 0 {
                    accumulated.append(buffer, count: readCount)
                    if accumulated.contains(0x0A) { break }
                } else {
                    break
                }
            } else if inStream.streamStatus == .error || inStream.streamStatus == .atEnd {
                break
            } else {
                Thread.sleep(forTimeInterval: 0.01)
            }
        }

        if accumulated.isEmpty { return nil }
        return try? JSONDecoder().decode(TelemetrySnapshot.self, from: accumulated)
    }

    @objc func refreshTelemetry() {
        DispatchQueue.global(qos: .utility).async {
            let snap = self.sendIPCCommand("\"GetSnapshot\"")
            DispatchQueue.main.async {
                if let s = snap {
                    self.statusItem.button?.title = String(
                        format: "⚛ %.0f%% CPU · %.1f TeV",
                        s.system_cpu_pct,
                        s.beam_energy_tev
                    )
                    self.stateMenuItem.title = String(
                        format: "● Engine RUNNING (%d/%d Threads @ %d%% CPU Intensity | Uptime %ds)",
                        s.active_threads, s.max_threads, s.intensity_pct, s.uptime_secs
                    )
                    self.cpuMenuItem.title = String(
                        format: "Consumption: %.1f%% Sys CPU | %.1f kH/s | %.2f GFLOPS | RAM: %d MB",
                        s.system_cpu_pct, s.hashes_per_sec / 1000.0, s.gflops, s.memory_used_mb
                    )
                    self.beamMenuItem.title = String(
                        format: "Beam: %.3f TeV (Peak %.2f TeV) | γ = %.1f | %.0f collisions/s",
                        s.beam_energy_tev, s.peak_energy_tev, s.lorentz_gamma, s.collisions_per_sec
                    )
                    self.quantumMenuItem.title = String(
                        format: "Quantum 5σ: %.1f%% | Next: %@ (%.2fσ/5σ) | Discovered: %d/12",
                        s.quantum_proximity_pct, s.next_target_symbol, s.next_target_sigma_progress, s.discoveries.count
                    )
                } else {
                    self.statusItem.button?.title = "⚛ OFF · 0% CPU"
                    self.stateMenuItem.title = "○ Engine Status: STOPPED (0% CPU Consumption)"
                    self.cpuMenuItem.title = "Consumption: 0.0% CPU | 0.0 kH/s | Engine Idle"
                    self.beamMenuItem.title = "Beam: Offline (Click 'Open All 3 Terminals' to start)"
                    self.quantumMenuItem.title = "Quantum Research Paused — Zero Background Resource Usage"
                }
            }
        }
    }

    func runAppleScript(_ script: String) {
        let task = Process()
        task.launchPath = "/usr/bin/osascript"
        task.arguments = ["-e", script]
        try? task.run()
    }

    func openTerminalWindow(titleTag: String, shellCmd: String) {
        let fullCmd = "printf '\\\\e]0;\(titleTag)\\\\a'; cd '\(workDir)' && \(shellCmd)"
        let escaped = fullCmd
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
        let script = """
        tell application "Terminal"
            activate
            do script "\(escaped)"
        end tell
        """
        runAppleScript(script)
    }

    @objc func openAllThreeTerminals() {
        openTerminal1Engine()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.55) {
            self.openTerminal2Viz()
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.05) {
            self.openTerminal3Cli()
        }
    }

    @objc func openTerminal1Engine() {
        let cmd = "'\(binDir)/collider-engine' --host \(host) --port \(port) --threads 2 --intensity 70"
        openTerminalWindow(titleTag: "LOCAL_COLLIDER_WIN_1_ENGINE", shellCmd: cmd)
    }

    @objc func openTerminal2Viz() {
        let cmd = "'\(binDir)/collider-viz' --host \(host) --port \(port)"
        openTerminalWindow(titleTag: "LOCAL_COLLIDER_WIN_2_VIZ", shellCmd: cmd)
    }

    @objc func openTerminal3Cli() {
        let cliBin = "'\(binDir)/local-collider'"
        let cmd = "\(cliBin) status; echo ''; \(cliBin) codex; echo ''; echo '--- Quick CLI Commands: ---'; echo '  \(cliBin) status'; echo '  \(cliBin) tune --mode quantum --auto-lock-rf --burst'; echo '  \(cliBin) codex'; exec $SHELL"
        openTerminalWindow(titleTag: "LOCAL_COLLIDER_WIN_3_CLI", shellCmd: cmd)
    }

    @objc func fireSupercriticalBurst() {
        DispatchQueue.global(qos: .userInitiated).async {
            _ = self.sendIPCCommand("\"TriggerPulseBurst\"")
            DispatchQueue.main.async { self.refreshTelemetry() }
        }
    }

    @objc func autoLockRfResonance() {
        DispatchQueue.global(qos: .userInitiated).async {
            _ = self.sendIPCCommand("\"LockOptimalHarmonic\"")
            DispatchQueue.main.async { self.refreshTelemetry() }
        }
    }

    @objc func setEcoCpuMode() {
        DispatchQueue.global(qos: .userInitiated).async {
            _ = self.sendIPCCommand("{\"Batch\":[{\"SetThreads\":1},{\"SetIntensity\":15}]}")
            DispatchQueue.main.async { self.refreshTelemetry() }
        }
    }

    @objc func setBalancedCpuMode() {
        DispatchQueue.global(qos: .userInitiated).async {
            _ = self.sendIPCCommand("{\"Batch\":[{\"SetThreads\":2},{\"SetIntensity\":65}]}")
            DispatchQueue.main.async { self.refreshTelemetry() }
        }
    }

    @objc func setOverdriveCpuMode() {
        DispatchQueue.global(qos: .userInitiated).async {
            _ = self.sendIPCCommand("{\"Batch\":[{\"SetThreads\":6},{\"SetIntensity\":95}]}")
            DispatchQueue.main.async { self.refreshTelemetry() }
        }
    }

    @objc func stopAllAccelerationAndCloseTerminals() {
        // 1. Send clean IPC Shutdown so collider-engine saves state to disk
        _ = sendIPCCommand("\"Shutdown\"")
        Thread.sleep(forTimeInterval: 0.15)

        // 2. Ensure all collider-engine and collider-viz processes are terminated (0% CPU guarantee)
        let killEngine = Process()
        killEngine.launchPath = "/usr/bin/pkill"
        killEngine.arguments = ["-f", "collider-engine"]
        try? killEngine.run()
        killEngine.waitUntilExit()

        let killViz = Process()
        killViz.launchPath = "/usr/bin/pkill"
        killViz.arguments = ["-f", "collider-viz"]
        try? killViz.run()
        killViz.waitUntilExit()

        // 3. Close any Terminal windows tagged with LOCAL_COLLIDER_WIN_
        let closeScript = """
        tell application "Terminal"
            close (every window whose name contains "LOCAL_COLLIDER_WIN_" or name contains "collider-engine" or name contains "collider-viz")
        end tell
        """
        runAppleScript(closeScript)

        refreshTelemetry()
    }

    @objc func quitAndStopEverything() {
        stopAllAccelerationAndCloseTerminals()
        NSApplication.shared.terminate(nil)
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let delegate = MenuBarDelegate()
app.delegate = delegate
app.run()
