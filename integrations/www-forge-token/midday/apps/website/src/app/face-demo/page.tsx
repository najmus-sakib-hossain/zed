"use client";

import { DxAiFace } from "@/components/dx-ai-face";
import { Button } from "@midday/ui/button";
import { useState } from "react";

export default function FaceDemoPage() {
  const [selectedEmotion, setSelectedEmotion] = useState("happy");
  const emotions = ["happy", "neutral", "excited", "surprised", "thinking"];

  return (
    <div className="min-h-screen bg-background p-8">
      <div className="max-w-4xl mx-auto space-y-12">
        <div className="text-center space-y-4">
          <h1 className="text-4xl font-bold">DX AI Agent Face Demo</h1>
          <p className="text-muted-foreground">
            Interactive face with blinking eyes and extensible emotion system
          </p>
        </div>

        <div className="grid gap-8">
          <div className="border border-border rounded-lg p-8 space-y-4">
            <h2 className="text-2xl font-semibold">Interactive Demo</h2>
            <div className="flex flex-col items-center gap-6">
              <DxAiFace size={280} emotion={selectedEmotion} interactive={true} />
              <div className="flex flex-wrap justify-center gap-2">
                {emotions.map((emotion) => (
                  <Button
                    key={emotion}
                    variant={selectedEmotion === emotion ? "default" : "outline"}
                    onClick={() => setSelectedEmotion(emotion)}
                    className="capitalize"
                  >
                    {emotion}
                  </Button>
                ))}
              </div>
            </div>
          </div>

          <div className="border border-border rounded-lg p-8 space-y-4">
            <h2 className="text-2xl font-semibold">Sizes</h2>
            <div className="flex items-center justify-around gap-8 flex-wrap">
              <div className="text-center space-y-2">
                <DxAiFace size={80} />
                <p className="text-sm text-muted-foreground">Small (80px)</p>
              </div>
              <div className="text-center space-y-2">
                <DxAiFace size={150} />
                <p className="text-sm text-muted-foreground">Medium (150px)</p>
              </div>
              <div className="text-center space-y-2">
                <DxAiFace size={220} />
                <p className="text-sm text-muted-foreground">Large (220px)</p>
              </div>
            </div>
          </div>

          <div className="border border-border rounded-lg p-8 space-y-4">
            <h2 className="text-2xl font-semibold">Interactions</h2>
            <div className="space-y-6">
              <div className="flex flex-col items-center gap-4">
                <DxAiFace size={200} />
                <div className="space-y-2 text-center">
                  <p className="font-medium">Try these interactions:</p>
                  <ul className="text-sm text-muted-foreground space-y-1">
                    <li>• <span className="font-medium">Move your mouse</span> - Eyes follow the cursor</li>
                    <li>• <span className="font-medium">Hover</span> - Face scales up with enhanced glow</li>
                    <li>• <span className="font-medium">Wait</span> - Eyes blink automatically every 3-5 seconds</li>
                  </ul>
                </div>
              </div>
            </div>
          </div>

          <div className="border border-border rounded-lg p-8 space-y-4">
            <h2 className="text-2xl font-semibold">Features</h2>
            <div className="grid md:grid-cols-2 gap-6">
              <div className="space-y-2">
                <h3 className="font-medium">Auto-blinking</h3>
                <p className="text-sm text-muted-foreground">
                  Eyes blink naturally every 3-5 seconds with smooth animation
                </p>
              </div>
              <div className="space-y-2">
                <h3 className="font-medium">Mouse tracking</h3>
                <p className="text-sm text-muted-foreground">
                  Eyes follow cursor movement with spring physics (limited range)
                </p>
              </div>
              <div className="space-y-2">
                <h3 className="font-medium">Extensible emotions</h3>
                <p className="text-sm text-muted-foreground">
                  Add new emotions through registry without changing component code
                </p>
              </div>
              <div className="space-y-2">
                <h3 className="font-medium">Smooth animations</h3>
                <p className="text-sm text-muted-foreground">
                  All transitions use Framer Motion with spring physics
                </p>
              </div>
            </div>
          </div>

          <div className="border border-border rounded-lg p-8 space-y-4">
            <h2 className="text-2xl font-semibold">Auto Emote Mode</h2>
            <div className="flex flex-col items-center gap-4">
              <DxAiFace size={200} autoEmote={true} />
              <p className="text-sm text-muted-foreground text-center">
                This face automatically changes emotions every 5 seconds
              </p>
            </div>
          </div>

          <div className="border border-border rounded-lg p-8 space-y-4">
            <h2 className="text-2xl font-semibold">Multiple Faces</h2>
            <div className="flex flex-wrap items-center justify-center gap-8">
              {emotions.map((emotion, i) => (
                <div key={i} className="text-center space-y-2">
                  <DxAiFace size={120} emotion={emotion} />
                  <p className="text-xs text-muted-foreground capitalize">{emotion}</p>
                </div>
              ))}
            </div>
            <p className="text-sm text-muted-foreground text-center">
              Each face has independent blinking and can show different emotions
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
