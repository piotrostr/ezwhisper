import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Config {
	elevenlabs_api_key: string;
	anthropic_api_key: string;
	language: string;
	auto_enter: boolean;
	cleanup: boolean;
	translate: boolean;
	device_index: number | null;
}

interface AudioDevice {
	index: number;
	name: string;
}

interface LogEntry {
	timestamp: string;
	level: string;
	message: string;
}

type AppStatus = "Idle" | "Recording" | "Transcribing";

const defaultConfig: Config = {
	elevenlabs_api_key: "",
	anthropic_api_key: "",
	language: "auto",
	auto_enter: true,
	cleanup: false,
	translate: false,
	device_index: null,
};

function App() {
	const [config, setConfig] = useState<Config>(defaultConfig);
	const [devices, setDevices] = useState<AudioDevice[]>([]);
	const [status, setStatus] = useState<AppStatus>("Idle");
	const [logs, setLogs] = useState<LogEntry[]>([]);
	const [saved, setSaved] = useState(false);
	const [showLogs, setShowLogs] = useState(false);
	const logsEndRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		loadConfig();
		loadDevices();
		loadStatus();
		loadLogs();

		// Listen for status changes
		const unlisten = listen<AppStatus>("status-changed", (event) => {
			setStatus(event.payload);
			loadLogs(); // Refresh logs on status change
		});

		// Poll logs every 2 seconds
		const interval = setInterval(loadLogs, 2000);

		return () => {
			unlisten.then((fn) => fn());
			clearInterval(interval);
		};
	}, []);

	useEffect(() => {
		// Auto-scroll logs
		logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [logs]);

	async function loadConfig() {
		try {
			const loaded = await invoke<Config>("get_config");
			setConfig(loaded);
		} catch (e) {
			console.error("Failed to load config:", e);
		}
	}

	async function loadDevices() {
		try {
			const deviceList = await invoke<AudioDevice[]>("list_audio_devices");
			setDevices(deviceList);
		} catch (e) {
			console.error("Failed to load devices:", e);
		}
	}

	async function loadStatus() {
		try {
			const s = await invoke<AppStatus>("get_status");
			setStatus(s);
		} catch (e) {
			console.error("Failed to load status:", e);
		}
	}

	async function loadLogs() {
		try {
			const l = await invoke<LogEntry[]>("get_logs");
			setLogs(l);
		} catch (e) {
			console.error("Failed to load logs:", e);
		}
	}

	async function saveConfig() {
		try {
			await invoke("save_config", { config });
			setSaved(true);
			setTimeout(() => setSaved(false), 2000);
		} catch (e) {
			console.error("Failed to save config:", e);
		}
	}

	function updateConfig<K extends keyof Config>(key: K, value: Config[K]) {
		setConfig((prev) => ({ ...prev, [key]: value }));
	}

	const statusColor = {
		Idle: "#2a2a2a",
		Recording: "#dc2626",
		Transcribing: "#2563eb",
	};

	return (
		<div style={{ padding: "20px", maxWidth: "400px", margin: "0 auto" }}>
			<h1 style={{ fontSize: "24px", marginBottom: "20px" }}>ezwhisper</h1>

			<div style={{ marginBottom: "20px" }}>
				<div
					style={{
						padding: "10px",
						background: statusColor[status],
						borderRadius: "8px",
						textAlign: "center",
						transition: "background 0.2s",
					}}
				>
					Status: <strong>{status}</strong>
				</div>
			</div>

			<div style={{ marginBottom: "16px" }}>
				<label style={{ display: "block", marginBottom: "4px" }}>
					ElevenLabs API Key
				</label>
				<input
					type="password"
					value={config.elevenlabs_api_key}
					onChange={(e) => updateConfig("elevenlabs_api_key", e.target.value)}
					style={{
						width: "100%",
						padding: "8px",
						background: "#2a2a2a",
						border: "1px solid #444",
						borderRadius: "4px",
						color: "#e0e0e0",
					}}
				/>
			</div>

			<div style={{ marginBottom: "16px" }}>
				<label style={{ display: "block", marginBottom: "4px" }}>
					Anthropic API Key (optional)
				</label>
				<input
					type="password"
					value={config.anthropic_api_key}
					onChange={(e) => updateConfig("anthropic_api_key", e.target.value)}
					style={{
						width: "100%",
						padding: "8px",
						background: "#2a2a2a",
						border: "1px solid #444",
						borderRadius: "4px",
						color: "#e0e0e0",
					}}
				/>
			</div>

			<div style={{ marginBottom: "16px" }}>
				<label style={{ display: "block", marginBottom: "4px" }}>
					Audio Device
				</label>
				<select
					value={config.device_index ?? ""}
					onChange={(e) =>
						updateConfig(
							"device_index",
							e.target.value ? parseInt(e.target.value) : null,
						)
					}
					style={{
						width: "100%",
						padding: "8px",
						background: "#2a2a2a",
						border: "1px solid #444",
						borderRadius: "4px",
						color: "#e0e0e0",
					}}
				>
					<option value="">Default</option>
					{devices.map((d) => (
						<option key={d.index} value={d.index}>
							{d.name}
						</option>
					))}
				</select>
			</div>

			<div style={{ marginBottom: "16px" }}>
				<label style={{ display: "block", marginBottom: "4px" }}>
					Language
				</label>
				<select
					value={config.language}
					onChange={(e) => updateConfig("language", e.target.value)}
					style={{
						width: "100%",
						padding: "8px",
						background: "#2a2a2a",
						border: "1px solid #444",
						borderRadius: "4px",
						color: "#e0e0e0",
					}}
				>
					<option value="auto">Auto-detect</option>
					<option value="en">English</option>
					<option value="pl">Polish</option>
					<option value="de">German</option>
					<option value="fr">French</option>
					<option value="es">Spanish</option>
				</select>
			</div>

			<div style={{ marginBottom: "12px" }}>
				<label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
					<input
						type="checkbox"
						checked={config.auto_enter}
						onChange={(e) => updateConfig("auto_enter", e.target.checked)}
					/>
					Auto-press Enter after paste
				</label>
			</div>

			<div style={{ marginBottom: "12px" }}>
				<label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
					<input
						type="checkbox"
						checked={config.translate}
						onChange={(e) => updateConfig("translate", e.target.checked)}
					/>
					Translate to English (requires Anthropic key)
				</label>
			</div>

			<div style={{ marginBottom: "20px" }}>
				<label style={{ display: "flex", alignItems: "center", gap: "8px" }}>
					<input
						type="checkbox"
						checked={config.cleanup}
						onChange={(e) => updateConfig("cleanup", e.target.checked)}
					/>
					Clean up grammar (requires Anthropic key)
				</label>
			</div>

			<button
				onClick={saveConfig}
				style={{
					width: "100%",
					padding: "12px",
					background: saved ? "#2d5a2d" : "#3b82f6",
					border: "none",
					borderRadius: "8px",
					color: "white",
					fontSize: "16px",
					cursor: "pointer",
					marginBottom: "12px",
				}}
			>
				{saved ? "Saved!" : "Save Settings"}
			</button>

			<button
				onClick={() => setShowLogs(!showLogs)}
				style={{
					width: "100%",
					padding: "8px",
					background: "#333",
					border: "1px solid #444",
					borderRadius: "8px",
					color: "#888",
					fontSize: "14px",
					cursor: "pointer",
				}}
			>
				{showLogs ? "Hide Logs" : "Show Logs"}
			</button>

			{showLogs && (
				<div
					style={{
						marginTop: "12px",
						background: "#1a1a1a",
						border: "1px solid #333",
						borderRadius: "8px",
						padding: "8px",
						maxHeight: "200px",
						overflowY: "auto",
						fontSize: "11px",
						fontFamily: "monospace",
					}}
				>
					{logs.length === 0 ? (
						<div style={{ color: "#666" }}>No logs yet</div>
					) : (
						logs.map((log, i) => (
							<div
								key={i}
								style={{
									color:
										log.level === "ERROR"
											? "#ef4444"
											: log.level === "WARN"
												? "#f59e0b"
												: "#888",
									marginBottom: "2px",
								}}
							>
								<span style={{ color: "#555" }}>{log.timestamp}</span>{" "}
								{log.message}
							</div>
						))
					)}
					<div ref={logsEndRef} />
				</div>
			)}

			<p
				style={{
					marginTop: "16px",
					fontSize: "12px",
					color: "#666",
					textAlign: "center",
				}}
			>
				Click Logitech button to start/stop recording
			</p>
		</div>
	);
}

export default App;
