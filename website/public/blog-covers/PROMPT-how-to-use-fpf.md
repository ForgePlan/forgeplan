# Cover prompt: how-to-use-fpf (How to start using FPF - 3-step protocol)

## Slugs
- RU: `how-to-use-fpf`
- EN: `how-to-use-fpf`

## Output filename
`/website/public/blog-covers/how-to-use-fpf.webp` (1600x900, ~95-97% quality)

## Prompt (copy-paste to image generator)

A minimalist isometric workflow diagram on a dark navy background (#0F1419). Three connected elements arranged horizontally with subtle orange (#FF6B35) connecting arrows: (1) a stack of markdown document icons on the left labeled with a small ".md" badge, (2) a stylized chat interface in the middle showing two abstract dialogue bubbles with no readable text, (3) a small "card" component on the right with clean grid lines suggesting a structured output (Name Card format). The overall feel: clean engineering schematic meets editorial illustration. Thin orange line work, dark navy fills, white pinpoints of light at connection nodes. No text labels visible. Off-center composition leaving space on the right third for title overlay. Subtle grid pattern in the background at 5% opacity.

## Style refs
- Companion to `what-is-fpf.webp` and `where-fpf-helps.webp` (same series, same palette)
- Avoid: stock-photo office, AI brain illustrations, gear icons, generic productivity visuals
- Avoid: any specific tool branding (no ChatGPT/Claude logos)

## After generation

1. Save PNG -> `/tmp/how-to-use-fpf-raw.png`
2. Convert: `cwebp -q 95 -m 6 /tmp/how-to-use-fpf-raw.png -o website/public/blog-covers/how-to-use-fpf.webp`
3. Verify: file size 80-300KB, dims 1600x900 (or close to 16:9)
