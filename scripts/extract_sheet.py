#!/usr/bin/env python3
"""
One-time script to extract a clean sprite sheet from the annotated reference image.

The annotated image (533×800) contains 8 rows × 6 columns of ~70×62px thumbnails
with text labels between rows. We extract 6 mood rows (skipping duplicates),
scale each cell up to 128×128, and add a 7th frame (duplicate of frame 0) to
create a complete 7×6 grid at 896×768.

Mood-to-row mapping (labels appear below each row):
  Row 0 → Neutral
  Row 1 → Happy
  Row 2 → Angry
  Row 4 → God Mode  (row 3 is an unused variant)
  Row 5 → Hurt Real Bad
  Row 6 → Thinking  (row 7 is unused)
"""

from PIL import Image

ANNOTATED_PATH = "/Users/tom/Desktop/9AD6F3B7-0721-44BC-BCDF-C82B924669CB.png"
OUTPUT_PATH = "assets/faces/default/sheet.png"

FRAME_SIZE = 128
FRAMES_PER_MOOD = 7

# Sprite row Y-ranges (top, bottom) detected from alpha analysis
SPRITE_ROWS = [
    (21, 83),    # row 0
    (114, 175),  # row 1
    (206, 268),  # row 2
    (299, 358),  # row 3
    (388, 454),  # row 4
    (483, 544),  # row 5
    (574, 635),  # row 6
    (665, 728),  # row 7
]

# Content column X-ranges (left, right)
CONTENT_COLS = [
    (34, 105),   # col 0
    (111, 181),  # col 1
    (187, 258),  # col 2
    (264, 334),  # col 3
    (339, 409),  # col 4
    (415, 485),  # col 5
]

# Which annotated rows map to each mood, in JSON ordinal order
MOOD_ROWS = [0, 1, 2, 4, 5, 6]  # neutral, happy, angry, god_mode, hurt_real_bad, thinking


def main():
    src = Image.open(ANNOTATED_PATH).convert("RGBA")
    print(f"Source: {src.size}")

    sheet_w = FRAMES_PER_MOOD * FRAME_SIZE  # 896
    sheet_h = len(MOOD_ROWS) * FRAME_SIZE   # 768
    sheet = Image.new("RGBA", (sheet_w, sheet_h), (0, 0, 0, 0))

    for mood_idx, src_row in enumerate(MOOD_ROWS):
        ry1, ry2 = SPRITE_ROWS[src_row]
        for col_idx, (cx1, cx2) in enumerate(CONTENT_COLS):
            cell = src.crop((cx1, ry1, cx2, ry2))
            cell = cell.resize((FRAME_SIZE, FRAME_SIZE), Image.LANCZOS)
            dst_x = col_idx * FRAME_SIZE
            dst_y = mood_idx * FRAME_SIZE
            sheet.paste(cell, (dst_x, dst_y))

        # 7th frame: duplicate of frame 0 to complete the animation loop
        first_cx1, first_cx2 = CONTENT_COLS[0]
        first_cell = src.crop((first_cx1, ry1, first_cx2, ry2))
        first_cell = first_cell.resize((FRAME_SIZE, FRAME_SIZE), Image.LANCZOS)
        sheet.paste(first_cell, (6 * FRAME_SIZE, mood_idx * FRAME_SIZE))

    sheet.save(OUTPUT_PATH)
    print(f"Output: {OUTPUT_PATH} ({sheet.size})")
    print(f"  {len(MOOD_ROWS)} moods × {FRAMES_PER_MOOD} frames × {FRAME_SIZE}×{FRAME_SIZE}px")


if __name__ == "__main__":
    main()
