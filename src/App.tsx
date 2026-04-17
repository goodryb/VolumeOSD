import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Volume2, VolumeX, Volume1 } from "lucide-react";
import "./App.css";

interface VolumePayload {
  volume: number;
  muted: boolean;
}

function App() {
  const [volume, setVolume] = useState(0);
  const [muted, setMuted] = useState(false);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    // Listen for volume change events from Rust
    const unlistenPromise = listen<VolumePayload>("volume-changed", (event) => {
      setVolume(event.payload.volume);
      setMuted(event.payload.muted);
      setVisible(true);
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    if (visible) {
      // Auto-hide the OSD after 2 seconds
      const timer = setTimeout(async () => {
        setVisible(false);
        const window = getCurrentWindow();
        await window.hide();
      }, 2000);
      return () => clearTimeout(timer);
    }
  }, [visible, volume, muted]);

  const getIcon = () => {
    if (muted || volume === 0) return <VolumeX size={20} />;
    if (volume < 50) return <Volume1 size={20} />;
    return <Volume2 size={20} />;
  };

  return (
    <div className={`osd-container ${visible ? "visible" : "hidden"}`}>
      <div className="volume-icon">{getIcon()}</div>
      <div className="progress-bar-bg">
        <div
          className="progress-bar-fill"
          style={{ width: `${muted ? 0 : volume}%` }}
        ></div>
      </div>
      <div className="volume-text">{muted ? "Muted" : `${volume}%`}</div>
    </div>
  );
}

export default App;
