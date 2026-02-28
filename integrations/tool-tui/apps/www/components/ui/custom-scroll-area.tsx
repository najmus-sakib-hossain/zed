"use client"

import * as React from "react"
import { ScrollArea as ScrollAreaPrimitive } from "radix-ui"
import { motion, useSpring, useTransform, MotionValue } from "framer-motion"

import { cn } from "@/lib/utils"

function ScrollArea({
  className,
  children,
  ...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.Root>) {
  return (
    <ScrollAreaPrimitive.Root
      data-slot="scroll-area"
      className={cn("relative overflow-hidden", className)}
      {...props}
    >
      <ScrollAreaPrimitive.Viewport
        data-slot="scroll-area-viewport"
        className="focus-visible:ring-ring/50 size-full rounded-[inherit] transition-[color,box-shadow] outline-none focus-visible:ring-[3px] focus-visible:outline-1"
      >
        {children}
      </ScrollAreaPrimitive.Viewport>
      <ScrollBar />
      <ScrollAreaPrimitive.Corner />
    </ScrollAreaPrimitive.Root>
  )
}

function ScrollBar({
  className,
  orientation = "vertical",
  ...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>) {
  const [isScrolling, setIsScrolling] = React.useState(false)
  const [isHovered, setIsHovered] = React.useState(false)
  const [isActive, setIsActive] = React.useState(false)
  const scrollTimeout = React.useRef<NodeJS.Timeout | null>(null)
  const thumbRef = React.useRef<HTMLDivElement>(null)
  
  const trackOpacity = useSpring(1, { stiffness: 300, damping: 30 })
  const width = useSpring(8, { stiffness: 400, damping: 25 })
  const opacity = useSpring(1, { stiffness: 300, damping: 30 })
  const thumbScale = useSpring(1, { stiffness: 500, damping: 20, bounce: 0.6 })
  const thumbWidth = useSpring(1, { stiffness: 500, damping: 20, bounce: 0.6 })
  const glowOpacity = useSpring(0, { stiffness: 300, damping: 30 })

  const widthPx = useTransform(width, (v) => `${v}px`)

  // Force consistent thumb height regardless of content
  React.useEffect(() => {
    if (thumbRef.current && orientation === "vertical") {
      const observer = new ResizeObserver(() => {
        if (thumbRef.current) {
          const computedHeight = thumbRef.current.offsetHeight
          if (computedHeight < 60) {
            thumbRef.current.style.height = "60px"
          } else if (computedHeight > 120) {
            thumbRef.current.style.height = "120px"
          }
        }
      })
      observer.observe(thumbRef.current)
      return () => observer.disconnect()
    }
  }, [orientation])

  const widthPx = useTransform(width, (v) => `${v}px`)

  const handleScroll = () => {
    setIsScrolling(true)
    trackOpacity.set(1)
    width.set(9)
    opacity.set(1)
    thumbScale.set(0.85)
    thumbWidth.set(1.3)
    glowOpacity.set(0.7)

    if (scrollTimeout.current) {
      clearTimeout(scrollTimeout.current)
    }

    scrollTimeout.current = setTimeout(() => {
      setIsScrolling(false)
      if (!isHovered && !isActive) {
        trackOpacity.set(1)
        width.set(8)
        opacity.set(1)
        thumbScale.set(1)
        thumbWidth.set(1)
        glowOpacity.set(0)
      }
    }, 1200)
  }

  const handleMouseEnter = () => {
    setIsHovered(true)
    trackOpacity.set(1)
    width.set(9)
    opacity.set(1)
    thumbScale.set(1.05)
    thumbWidth.set(1.2)
    glowOpacity.set(0.4)
  }

  const handleMouseLeave = () => {
    setIsHovered(false)
    if (!isScrolling && !isActive) {
      trackOpacity.set(1)
      width.set(8)
      opacity.set(1)
      thumbScale.set(1)
      thumbWidth.set(1)
      glowOpacity.set(0)
    }
  }

  const handleMouseDown = () => {
    setIsActive(true)
    trackOpacity.set(1)
    width.set(8)
    opacity.set(1)
    thumbScale.set(1)
    thumbWidth.set(1)
    glowOpacity.set(0)
  }

  const handleMouseUp = () => {
    setIsActive(false)
    if (isHovered) {
      trackOpacity.set(1)
      width.set(9)
      thumbScale.set(1.05)
      thumbWidth.set(1.2)
      glowOpacity.set(0.4)
    } else {
      trackOpacity.set(1)
      width.set(8)
      opacity.set(1)
      thumbScale.set(1)
      thumbWidth.set(1)
      glowOpacity.set(0)
    }
  }

  React.useEffect(() => {
    const handleGlobalMouseUp = () => {
      if (isActive) {
        handleMouseUp()
      }
    }
    
    window.addEventListener('mouseup', handleGlobalMouseUp)
    
    return () => {
      window.removeEventListener('mouseup', handleGlobalMouseUp)
      if (scrollTimeout.current) {
        clearTimeout(scrollTimeout.current)
      }
    }
  }, [isActive, isHovered])

  return (
    <ScrollAreaPrimitive.ScrollAreaScrollbar
      data-slot="scroll-area-scrollbar"
      orientation={orientation}
      onScroll={handleScroll}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onMouseDown={handleMouseDown}
      className={cn(
        "flex touch-none transition-colors select-none z-50 relative",
        orientation === "vertical" && "h-full border-l border-l-transparent p-1.5 pb-[120px]",
        orientation === "horizontal" && "flex-col border-t border-t-transparent p-1.5 pr-[120px]",
        className
      )}
      style={{
        minHeight: orientation === "vertical" ? "100%" : undefined,
        minWidth: orientation === "horizontal" ? "100%" : undefined,
      }}
      {...props}
    >
      {/* Track background */}
      <motion.div
        className="absolute inset-0 bg-secondary/75"
        style={{ opacity: trackOpacity } as any}
      />
      
      <motion.div
        className="relative flex-1 flex items-center justify-center"
        style={{ 
          width: orientation === "vertical" ? widthPx : undefined,
          height: orientation === "horizontal" ? widthPx : undefined,
          opacity
        } as any}
      >
        <motion.div
          className="absolute inset-0 min-h-[60px]"
          style={{
            scaleX: orientation === "vertical" ? thumbWidth : thumbScale,
            scaleY: orientation === "horizontal" ? thumbWidth : thumbScale,
          }}
        >
          <ScrollAreaPrimitive.ScrollAreaThumb
            ref={thumbRef}
            data-slot="scroll-area-thumb"
            className={cn(
              "relative flex-1 rounded-full transition-all duration-200 cursor-grab active:cursor-grabbing",
              isActive
                ? "bg-secondary" 
                : isScrolling || isHovered 
                ? "bg-secondary" 
                : "bg-secondary/90"
            )}
            style={{
              minHeight: "60px",
              maxHeight: "120px",
            }}
          />
        </motion.div>
        
        <motion.div
          className="absolute inset-0 bg-secondary/10 blur-md pointer-events-none rounded-full"
          style={{ opacity: glowOpacity } as any}
        />
      </motion.div>
    </ScrollAreaPrimitive.ScrollAreaScrollbar>
  )
}

export { ScrollArea, ScrollBar }
