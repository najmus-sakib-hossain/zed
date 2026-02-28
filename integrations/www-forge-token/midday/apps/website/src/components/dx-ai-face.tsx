"use client";

import { motion, useMotionValue, useSpring, useAnimation } from "motion/react";
import { useEffect, useState, useRef } from "react";

// Emotion system - easily extensible
interface Emotion {
  name: string;
  eyes: {
    shape: "semicircle" | "circle" | "line";
    top: string;
    scaleY: number;
  };
  mouth: {
    path: string;
  };
  eyebrows?: {
    left: string;
    right: string;
  };
}

const EMOTIONS: Record<string, Emotion> = {
  happy: {
    name: "happy",
    eyes: {
      shape: "circle",
      top: "38%",
      scaleY: 1,
    },
    mouth: {
      path: "M 38 58 Q 50 68 62 58",
    },
  },
  neutral: {
    name: "neutral",
    eyes: {
      shape: "circle",
      top: "38%",
      scaleY: 1,
    },
    mouth: {
      path: "M 40 60 Q 50 66 60 60",
    },
  },
  excited: {
    name: "excited",
    eyes: {
      shape: "circle",
      top: "36%",
      scaleY: 1,
    },
    mouth: {
      path: "M 35 56 Q 50 70 65 56",
    },
  },
  surprised: {
    name: "surprised",
    eyes: {
      shape: "circle",
      top: "36%",
      scaleY: 1.2,
    },
    mouth: {
      path: "M 47 60 Q 50 65 53 60 Q 50 55 47 60",
    },
  },
  thinking: {
    name: "thinking",
    eyes: {
      shape: "circle",
      top: "38%",
      scaleY: 1,
    },
    mouth: {
      path: "M 42 62 L 58 62",
    },
  },
  winking: {
    name: "winking",
    eyes: {
      shape: "circle",
      top: "38%",
      scaleY: 1,
    },
    mouth: {
      path: "M 38 58 Q 50 68 62 58",
    },
  },
  sleeping: {
    name: "sleeping",
    eyes: {
      shape: "line",
      top: "38%",
      scaleY: 0.2,
    },
    mouth: {
      path: "M 40 62 Q 50 64 60 62",
    },
  },
  dizzy: {
    name: "dizzy",
    eyes: {
      shape: "circle",
      top: "38%",
      scaleY: 1,
    },
    mouth: {
      path: "M 40 62 Q 50 60 60 62",
    },
  },
};

interface DxAiFaceProps {
  size?: number;
  emotion?: string;
  interactive?: boolean;
  autoEmote?: boolean;
  className?: string;
}

export function DxAiFace({
  size = 200,
  emotion = "happy",
  interactive = true,
  autoEmote = false,
  className = "",
}: DxAiFaceProps) {
  const [currentEmotion, setCurrentEmotion] = useState(emotion);
  const [isBlinking, setIsBlinking] = useState(false);
  const [isHovered, setIsHovered] = useState(false);
  const [isPressed, setIsPressed] = useState(false);
  const [isWinking, setIsWinking] = useState(false);
  const [isDragging, setIsDragging] = useState(false);

  const longPressTimer = useRef<NodeJS.Timeout | null>(null);
  const clickCount = useRef(0);
  const clickTimer = useRef<NodeJS.Timeout | null>(null);
  const shakeDetector = useRef({ lastX: 0, lastY: 0, shakeCount: 0 });
  const hoverTimer = useRef<NodeJS.Timeout | null>(null);

  const mouseX = useMotionValue(0);
  const mouseY = useMotionValue(0);
  const faceControls = useAnimation();

  // Better spring physics - eyes naturally return to center
  const smoothX = useSpring(mouseX, { 
    stiffness: 400, 
    damping: 30, 
    mass: 0.8,
    restDelta: 0.001,
    restSpeed: 0.001,
  });
  const smoothY = useSpring(mouseY, { 
    stiffness: 400, 
    damping: 30, 
    mass: 0.8,
    restDelta: 0.001,
    restSpeed: 0.001,
  });

  // Fast auto-blink with bounce effect
  useEffect(() => {
    const blinkInterval = setInterval(() => {
      setIsBlinking(true);
      // Fast blink with bounce
      setTimeout(() => setIsBlinking(false), 150);
    }, 3000 + Math.random() * 2000);

    return () => clearInterval(blinkInterval);
  }, []);

  // Auto emotion change (if enabled)
  useEffect(() => {
    if (!autoEmote) return;

    const emotionInterval = setInterval(() => {
      const emotions = Object.keys(EMOTIONS);
      const randomEmotion = emotions[Math.floor(Math.random() * emotions.length)];
      if (randomEmotion) {
        setCurrentEmotion(randomEmotion);
      }
    }, 5000);

    return () => clearInterval(emotionInterval);
  }, [autoEmote]);

  // Keyboard interactions
  useEffect(() => {
    const handleKeyPress = (e: KeyboardEvent) => {
      if (!interactive) return;
      
      switch(e.key) {
        case ' ':
          setIsBlinking(true);
          setTimeout(() => setIsBlinking(false), 180);
          break;
        case 'w':
          setIsWinking(true);
          setTimeout(() => setIsWinking(false), 500);
          break;
        case 'e':
          setCurrentEmotion("excited");
          setTimeout(() => setCurrentEmotion("happy"), 1000);
          break;
        case 's':
          setCurrentEmotion("surprised");
          setTimeout(() => setCurrentEmotion("happy"), 1000);
          break;
        case 't':
          setCurrentEmotion("thinking");
          setTimeout(() => setCurrentEmotion("happy"), 1000);
          break;
        case 'ArrowUp':
        case 'ArrowDown':
        case 'ArrowLeft':
        case 'ArrowRight':
          // Arrow key navigation
          faceControls.start({
            rotate: [0, e.key === 'ArrowLeft' ? -10 : e.key === 'ArrowRight' ? 10 : 0, 0],
            y: [0, e.key === 'ArrowUp' ? -10 : e.key === 'ArrowDown' ? 10 : 0, 0],
            transition: { duration: 0.3 },
          });
          break;
      }
    };

    window.addEventListener('keydown', handleKeyPress);
    return () => window.removeEventListener('keydown', handleKeyPress);
  }, [interactive, faceControls]);

  const handleMouseMove = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!interactive || isDragging) return;

    const rect = event.currentTarget.getBoundingClientRect();
    const x = event.clientX - rect.left - rect.width / 2;
    const y = event.clientY - rect.top - rect.height / 2;
    
    // Shake detection
    const deltaX = Math.abs(x - shakeDetector.current.lastX);
    const deltaY = Math.abs(y - shakeDetector.current.lastY);
    
    if (deltaX > 50 || deltaY > 50) {
      shakeDetector.current.shakeCount++;
      if (shakeDetector.current.shakeCount > 5) {
        setCurrentEmotion("dizzy");
        faceControls.start({
          rotate: [0, -5, 5, -5, 5, 0],
          transition: { duration: 0.5 },
        });
        setTimeout(() => setCurrentEmotion("happy"), 1500);
        shakeDetector.current.shakeCount = 0;
      }
    }
    
    shakeDetector.current.lastX = x;
    shakeDetector.current.lastY = y;
    
    // Constrain eye movement to stay inside circle
    // Calculate distance from center
    const distance = Math.sqrt(x * x + y * y);
    const maxDistance = size * 0.25; // Max distance eyes can move from center
    
    let constrainedX = x;
    let constrainedY = y;
    
    if (distance > maxDistance) {
      // Constrain to circle boundary
      const angle = Math.atan2(y, x);
      constrainedX = Math.cos(angle) * maxDistance;
      constrainedY = Math.sin(angle) * maxDistance;
    }
    
    // Apply constrained movement
    mouseX.set(constrainedX * 0.3);
    mouseY.set(constrainedY * 0.3);
  };

  const handleMouseLeave = () => {
    // Smooth return to center
    mouseX.set(0);
    mouseY.set(0);
    setIsHovered(false);
    setIsPressed(false);
    setIsDragging(false);
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
    }
    if (hoverTimer.current) {
      clearTimeout(hoverTimer.current);
    }
  };

  const handleMouseEnter = () => {
    setIsHovered(true);
    
    // Long hover interaction
    hoverTimer.current = setTimeout(() => {
      setIsWinking(true);
      setTimeout(() => setIsWinking(false), 500);
    }, 3000);
  };

  const handleMouseDown = (e: React.MouseEvent) => {
    setIsPressed(true);
    setIsDragging(true);
    
    // Long press detection
    longPressTimer.current = setTimeout(() => {
      setCurrentEmotion("sleeping");
      faceControls.start({
        scale: [1, 0.95, 1],
        transition: { duration: 0.5 },
      });
      setTimeout(() => setCurrentEmotion("happy"), 2000);
    }, 1000);
  };

  const handleMouseUp = () => {
    setIsPressed(false);
    setIsDragging(false);
    if (longPressTimer.current) {
      clearTimeout(longPressTimer.current);
    }
  };

  const handleClick = () => {
    clickCount.current += 1;

    if (clickTimer.current) {
      clearTimeout(clickTimer.current);
    }

    clickTimer.current = setTimeout(() => {
      if (clickCount.current === 1) {
        // Single click - fixed: only 2 keyframes for spring
        setCurrentEmotion("excited");
        faceControls.start({
          scale: 1.1,
          transition: { type: "spring", stiffness: 500, damping: 15 },
        }).then(() => {
          faceControls.start({
            scale: 1,
            transition: { type: "spring", stiffness: 500, damping: 15 },
          });
        });
        setTimeout(() => setCurrentEmotion("happy"), 800);
      } else if (clickCount.current === 2) {
        // Double click
        setIsWinking(true);
        faceControls.start({
          rotate: [0, -10, 10, 0],
          transition: { duration: 0.4 },
        });
        setTimeout(() => setIsWinking(false), 500);
      } else if (clickCount.current >= 3) {
        // Triple click - fixed: use sequential animations
        setCurrentEmotion("surprised");
        faceControls.start({
          scale: 1.15,
          rotate: 5,
          transition: { duration: 0.15 },
        }).then(() => {
          faceControls.start({
            scale: 0.95,
            rotate: -5,
            transition: { duration: 0.15 },
          });
        }).then(() => {
          faceControls.start({
            scale: 1,
            rotate: 0,
            transition: { type: "spring", stiffness: 400, damping: 20 },
          });
        });
        setTimeout(() => setCurrentEmotion("happy"), 1000);
      }
      clickCount.current = 0;
    }, 300);
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setCurrentEmotion("thinking");
    faceControls.start({
      rotate: [0, 15, -15, 0],
      transition: { duration: 0.5 },
    });
    setTimeout(() => setCurrentEmotion("happy"), 1000);
  };

  const handleDoubleClick = () => {
    setIsWinking(true);
    setTimeout(() => setIsWinking(false), 500);
  };

  const handleWheel = (e: React.WheelEvent) => {
    if (!interactive) return;
    
    // Scroll interaction
    const direction = e.deltaY > 0 ? 1 : -1;
    faceControls.start({
      rotate: [0, direction * 20, 0],
      transition: { duration: 0.4, type: "spring" },
    });
    
    if (Math.abs(e.deltaY) > 50) {
      setCurrentEmotion("dizzy");
      setTimeout(() => setCurrentEmotion("happy"), 800);
    }
  };

  const handleDragStart = () => {
    setIsDragging(true);
    setCurrentEmotion("surprised");
  };

  const handleDragEnd = () => {
    setIsDragging(false);
    setCurrentEmotion("happy");
    faceControls.start({
      x: 0,
      y: 0,
      transition: { type: "spring", stiffness: 300, damping: 20 },
    });
  };

  const activeEmotion = EMOTIONS[currentEmotion] || EMOTIONS.happy;

  return (
    <motion.div
      className={`relative ${className}`}
      style={{ width: size, height: size }}
      onMouseMove={handleMouseMove}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onMouseDown={handleMouseDown}
      onMouseUp={handleMouseUp}
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
      onWheel={handleWheel}
      drag={interactive}
      dragConstraints={{ left: 0, right: 0, top: 0, bottom: 0 }}
      dragElastic={0.2}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      animate={faceControls}
    >
      {/* Main face container */}
      <motion.div
        className="absolute inset-0 rounded-full bg-background border-2 border-border"
        animate={{
          scale: isHovered ? 1.02 : 1,
        }}
        transition={{ 
          type: "spring",
          stiffness: 300,
          damping: 25,
        }}
      >
        <svg
          viewBox="0 0 100 100"
          className="w-full h-full"
          style={{ overflow: "visible" }}
        >
          {/* Left Eye - Better centered */}
          <motion.g
            style={{
              x: smoothX,
              y: smoothY,
            }}
          >
            {isBlinking || (isWinking && Math.random() > 0.5) ? (
              // Blinking: fast with bounce effect
              <motion.path
                d="M 25 48 Q 35 54 45 48 Z"
                fill="currentColor"
                className="text-foreground"
                initial={{ scaleY: 1 }}
                animate={{ scaleY: 0.05 }}
                exit={{ scaleY: 1 }}
                transition={{ 
                  type: "spring",
                  stiffness: 800,
                  damping: 12,
                  mass: 0.3,
                }}
              />
            ) : (
              // Normal: full circle with subtle bounce when opening
              <motion.circle
                cx="35"
                cy="48"
                r="10"
                fill="currentColor"
                className="text-foreground"
                initial={{ scale: 0.9 }}
                animate={{ scale: 1 }}
                transition={{
                  type: "spring",
                  stiffness: 600,
                  damping: 10,
                }}
              />
            )}
          </motion.g>

          {/* Right Eye - Better centered */}
          <motion.g
            style={{
              x: smoothX,
              y: smoothY,
            }}
          >
            {isBlinking || isWinking ? (
              // Blinking: fast with bounce effect
              <motion.path
                d="M 55 48 Q 65 54 75 48 Z"
                fill="currentColor"
                className="text-foreground"
                initial={{ scaleY: 1 }}
                animate={{ scaleY: 0.05 }}
                exit={{ scaleY: 1 }}
                transition={{ 
                  type: "spring",
                  stiffness: 800,
                  damping: 12,
                  mass: 0.3,
                }}
              />
            ) : (
              // Normal: full circle with subtle bounce when opening
              <motion.circle
                cx="65"
                cy="48"
                r="10"
                fill="currentColor"
                className="text-foreground"
                initial={{ scale: 0.9 }}
                animate={{ scale: 1 }}
                transition={{
                  type: "spring",
                  stiffness: 600,
                  damping: 10,
                }}
              />
            )}
          </motion.g>
        </svg>
      </motion.div>
    </motion.div>
  );
}
