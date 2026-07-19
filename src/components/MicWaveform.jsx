import React from "react";
import { Mic } from "lucide-react";

const BAR_COUNT = 7;
const MIN_BAR_HEIGHT = 8;
const MAX_BAR_HEIGHT = 20;
const BAR_GAIN = 1.25;
// Analyser smoothing (0–1). Don't push above ~0.85 or bars feel dead.
const ANALYSER_SMOOTHING = 0.78;
// Bar follow speed (0–1). Higher = snappier, lower = floatier.
const BAR_LERP = 0.32;

function createIdleHeights() {
  return Array.from({ length: BAR_COUNT }, () => MIN_BAR_HEIGHT);
}

function frequencyBinsToHeights(bins) {
  return Array.from({ length: BAR_COUNT }, (_, index) => {
    const binIndex = Math.min(index * 2 + 1, bins.length - 1);
    const normalized = bins[binIndex] / 255;
    const boosted = Math.min(1, Math.pow(normalized, 0.55) * BAR_GAIN);
    return MIN_BAR_HEIGHT + boosted * (MAX_BAR_HEIGHT - MIN_BAR_HEIGHT);
  });
}

function stopMicCapture(stream, audioContext, animationFrame) {
  if (animationFrame !== null) {
    cancelAnimationFrame(animationFrame);
  }

  stream?.getTracks().forEach((track) => track.stop());

  if (audioContext && audioContext.state !== "closed") {
    void audioContext.close();
  }
}

export default function MicWaveform({ active }) {
  const [barHeights, setBarHeights] = React.useState(createIdleHeights);
  const [micBlocked, setMicBlocked] = React.useState(false);
  const sessionRef = React.useRef(0);

  React.useEffect(() => {
    if (!active) {
      setBarHeights(createIdleHeights());
      setMicBlocked(false);
      return;
    }

    sessionRef.current += 1;
    const session = sessionRef.current;

    let animationFrame = null;
    let stream = null;
    let audioContext = null;

    const start = async () => {
      try {
        stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        if (session !== sessionRef.current) {
          stopMicCapture(stream, null, null);
          return;
        }

        audioContext = new AudioContext();
        if (audioContext.state === "suspended") {
          await audioContext.resume();
        }
        if (session !== sessionRef.current) {
          stopMicCapture(stream, audioContext, null);
          return;
        }

        const source = audioContext.createMediaStreamSource(stream);
        const analyser = audioContext.createAnalyser();
        analyser.fftSize = 64;
        analyser.smoothingTimeConstant = ANALYSER_SMOOTHING;
        source.connect(analyser);

        const bins = new Uint8Array(analyser.frequencyBinCount);
        const smoothedHeights = createIdleHeights();

        const tick = () => {
          if (session !== sessionRef.current) {
            return;
          }

          analyser.getByteFrequencyData(bins);
          const targets = frequencyBinsToHeights(bins);

          for (let i = 0; i < BAR_COUNT; i += 1) {
            smoothedHeights[i] += (targets[i] - smoothedHeights[i]) * BAR_LERP;
          }

          setBarHeights([...smoothedHeights]);
          animationFrame = requestAnimationFrame(tick);
        };

        tick();
      } catch (error) {
        console.error("MicWaveform: microphone unavailable", error);
        if (session === sessionRef.current) {
          setMicBlocked(true);
          setBarHeights(createIdleHeights());
        }
      }
    };

    void start();

    return () => {
      sessionRef.current += 1;
      stopMicCapture(stream, audioContext, animationFrame);
    };
  }, [active]);

  if (micBlocked) {
    return (
      <div className="mic-waveform mic-waveform--blocked" aria-label="Microphone access denied">
        <Mic className="mic-waveform-fallback-icon" strokeWidth={2} aria-hidden="true" />
      </div>
    );
  }

  return (
    <div className="mic-waveform" aria-hidden="true">
      {barHeights.map((height, index) => (
        <span
          key={index}
          className="mic-waveform-bar"
          style={{ height: `${height}px` }}
        />
      ))}
    </div>
  );
}
