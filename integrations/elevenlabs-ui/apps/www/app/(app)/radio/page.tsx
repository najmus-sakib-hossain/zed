import { Metadata } from "next"

import { RadioSpeaker } from "./components/radio-speaker"

const title = "Cube.fm Radio"
const description = "Listen to Cube.fm's live radio stream"

export const metadata: Metadata = {
  title,
  description,
  openGraph: {
    images: [
      {
        url: `/og?title=${encodeURIComponent(
          title
        )}&description=${encodeURIComponent(description)}`,
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    images: [
      {
        url: `/og?title=${encodeURIComponent(
          title
        )}&description=${encodeURIComponent(description)}`,
      },
    ],
  },
}

export default function RadioPage() {
  return (
    <div className="flex flex-1 flex-col items-center justify-center overflow-hidden p-4">
      <div className="relative w-full max-w-2xl">
        <RadioSpeaker />
      </div>
    </div>
  )
}
