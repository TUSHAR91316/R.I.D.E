#!/usr/bin/env python3
"""
R.I.D.E — Microsoft Store Asset & Logo Generator
Creates high-fidelity master vector SVG and all required Microsoft Store MSIX/UWP visual assets.
Brand: R.I.D.E (Rust Intelligent Development Environment) by Byte-Labs
"""

import os
import math
from PIL import Image, ImageDraw, ImageFilter, ImageFont

ASSETS_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "Assets"))
os.makedirs(ASSETS_DIR, exist_ok=True)

# ---------------------------------------------------------------------------
# COLOR PALETTE
# ---------------------------------------------------------------------------
BG_DARK = (10, 13, 22, 255)         # #0A0D16 Deep Obsidian
BG_CARD = (15, 23, 42, 255)         # #0F172A Slate 900
BORDER_COLOR = (45, 60, 90, 200)    # Subtlest border glow
RUST_ORANGE = (255, 107, 0, 255)    # #FF6B00 Rust Core
RUST_AMBER = (255, 154, 34, 255)    # #FF9A22 High flame
RUST_DEEP = (217, 65, 0, 255)       # #D94100 Deep Rust
CYAN_GLOW = (0, 229, 255, 255)      # #00E5FF Neural Cyan
CYAN_DEEP = (14, 116, 144, 255)     # #0E7490 Blue 700
WHITE = (255, 255, 255, 255)
TEXT_MUTED = (148, 163, 184, 255)   # #94A3B8 Slate 400


def create_supersampled_icon(size=1024, unplated=False, with_bg_glow=True):
    """
    Renders the R.I.D.E master glyph at high resolution (1024x1024).
    The glyph features:
      - Stylized dual-rail Rust-orange vertical stem with Byte-Labs circuit notches
      - Aerodynamic looped head framing an intelligent neural diamond core
      - Dynamic forward-accelerating kick / thruster leg
      - Floating AI diamond core in cyan & electric plasma
    """
    canvas_size = 1024
    scale = canvas_size / 1024.0

    img = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    if not unplated:
        # Background squircle
        margin = int(40 * scale)
        radius = int(210 * scale)
        
        # Subtle outer atmospheric glow
        glow_layer = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
        glow_draw = ImageDraw.Draw(glow_layer)
        glow_draw.rounded_rectangle(
            [margin - 12, margin - 12, canvas_size - margin + 12, canvas_size - margin + 12],
            radius=radius + 8,
            fill=(255, 107, 0, 35)
        )
        glow_layer = glow_layer.filter(ImageFilter.GaussianBlur(18))
        img = Image.alpha_composite(img, glow_layer)
        draw = ImageDraw.Draw(img)

        # Base Squircle
        draw.rounded_rectangle(
            [margin, margin, canvas_size - margin, canvas_size - margin],
            radius=radius,
            fill=BG_DARK,
            outline=BORDER_COLOR,
            width=int(5 * scale)
        )

        # Ambient inner subtle gradient / grid lines
        grid_overlay = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
        grid_draw = ImageDraw.Draw(grid_overlay)
        # Radial orange-to-cyan glow center
        for r in range(350, 0, -25):
            alpha = int(22 * (1.0 - r / 350))
            grid_draw.ellipse(
                [512 - r, 512 - r, 512 + r, 512 + r],
                fill=(255, 107, 0, alpha)
            )
        for r in range(200, 0, -20):
            alpha = int(30 * (1.0 - r / 200))
            grid_draw.ellipse(
                [530 - r, 430 - r, 530 + r, 430 + r],
                fill=(0, 229, 255, alpha)
            )
        img = Image.alpha_composite(img, grid_overlay)
        draw = ImageDraw.Draw(img)

    # -------------------------------------------------------------
    # GLYPH GEOMETRY (Stylized "R")
    # Coordinates in 1024x1024 master space
    # -------------------------------------------------------------
    glyph_layer = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glyph_layer)

    # 1. Left Vertical Stem (Dual Rail Compiler Vector)
    # Rail 1 (Outer left)
    rail1 = [
        (220, 210), (290, 210),
        (290, 810), (220, 810)
    ]
    # Rail 2 (Inner left track)
    rail2 = [
        (315, 210), (365, 210),
        (365, 810), (315, 810)
    ]

    # Fill rails with gradient simulated slices
    for y in range(210, 810):
        t = (y - 210) / 600.0
        # Gradient: Rust Amber -> Orange -> Deep Rust
        r = int(255 - t * 40)
        g = int(165 - t * 105)
        b = int(40 - t * 30)
        gdraw.line([(220, y), (290, y)], fill=(r, g, b, 255))
        gdraw.line([(315, y), (365, y)], fill=(r, g, b, 230))

    # Circuit accents on left rail (Byte-Labs notches)
    notch_color = (0, 229, 255, 240)
    for ny in [320, 430, 540, 650]:
        gdraw.rectangle([210, ny - 6, 230, ny + 6], fill=notch_color)
        gdraw.ellipse([185, ny - 5, 195, ny + 5], fill=notch_color)
        gdraw.line([(195, ny), (210, ny)], fill=notch_color, width=3)

    # 2. Upper Bowl (Loop of the "R")
    # Outer curve points
    # Upper horizontal bar:
    gdraw.polygon([
        (365, 210), (620, 210),
        (750, 310), (790, 420),
        (750, 520), (620, 570),
        (365, 570), (365, 485),
        (600, 485), (680, 460),
        (700, 420), (680, 370),
        (600, 295), (365, 295)
    ], fill=RUST_AMBER)

    # Gradient overlay for upper loop to give metallic 3D sheen
    for y in range(210, 571, 3):
        t = (y - 210) / 360.0
        glow_r = int(255)
        glow_g = int(180 * (1.0 - t * 0.5))
        glow_b = int(30 + t * 40)
        # Highlight right arch
        x_start = max(365, int(620 - 200 * math.sin(t * math.pi)))
        x_end = min(790, int(620 + 170 * math.sin(t * math.pi)))
        if x_end > x_start:
            gdraw.line([(x_start, y), (x_end, y)], fill=(glow_r, glow_g, glow_b, 180), width=3)

    # 3. Dynamic Forward Thruster Leg (The kick of the "R")
    # Hyper-speed trajectory chevron
    leg_poly = [
        (480, 520),   # Start at underside of loop
        (600, 520),
        (800, 780),   # Right toe
        (825, 810),
        (715, 810),
        (560, 610),   # Inner elbow
        (460, 610)
    ]
    gdraw.polygon(leg_poly, fill=RUST_ORANGE)

    # Trailing light streak on leg
    streak_poly = [
        (580, 530),
        (610, 530),
        (825, 810),
        (795, 810)
    ]
    gdraw.polygon(streak_poly, fill=(255, 215, 0, 220))

    # Second energy kick bar (speed rail)
    speed_kick = [
        (670, 660),
        (710, 660),
        (825, 770),
        (825, 810),
        (790, 810)
    ]
    gdraw.polygon(speed_kick, fill=(255, 69, 0, 240))

    # 4. Neural Diamond Core (The "I" / Intelligence MoE heart)
    # Centered inside the open bowl of the R: center ~(520, 390)
    cx, cy = 520, 390
    diamond_h = 75
    diamond_w = 55

    # Outer diamond glow
    diamond_glow = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    dg_draw = ImageDraw.Draw(diamond_glow)
    for r in range(60, 0, -5):
        alpha = int(45 * (1.0 - r / 60))
        dg_draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(0, 229, 255, alpha))
    diamond_glow = diamond_glow.filter(ImageFilter.GaussianBlur(10))
    glyph_layer = Image.alpha_composite(glyph_layer, diamond_glow)
    gdraw = ImageDraw.Draw(glyph_layer)

    # 3D Diamond Facets:
    # Top facet
    gdraw.polygon([(cx, cy - diamond_h), (cx - diamond_w, cy), (cx, cy)], fill=(56, 189, 248, 255))
    # Right facet
    gdraw.polygon([(cx, cy - diamond_h), (cx + diamond_w, cy), (cx, cy)], fill=(125, 211, 252, 255))
    # Bottom-left facet
    gdraw.polygon([(cx, cy + diamond_h), (cx - diamond_w, cy), (cx, cy)], fill=(2, 132, 199, 255))
    # Bottom-right facet
    gdraw.polygon([(cx, cy + diamond_h), (cx + diamond_w, cy), (cx, cy)], fill=(14, 165, 233, 255))

    # White-hot neural core spark
    spark_h = 24
    spark_w = 16
    gdraw.polygon([(cx, cy - spark_h), (cx + spark_w, cy), (cx, cy + spark_h), (cx - spark_w, cy)], fill=(255, 255, 255, 255))

    # Horizontal synaptic beam passing through core
    gdraw.line([(cx - 140, cy), (cx + 140, cy)], fill=(0, 229, 255, 120), width=3)
    gdraw.line([(cx - 70, cy), (cx + 70, cy)], fill=(255, 255, 255, 200), width=2)
    # Neural beam dots
    gdraw.ellipse([cx - 145, cy - 5, cx - 135, cy + 5], fill=CYAN_GLOW)
    gdraw.ellipse([cx + 135, cy - 5, cx + 145, cy + 5], fill=CYAN_GLOW)

    # Composite glyph onto base image
    img = Image.alpha_composite(img, glyph_layer)

    return img


def create_wide_tile(width=1240, height=600):
    """
    Renders the Wide310x150 master tile (scaled up to 1240x600 for sharp rendering).
    Features the R.I.D.E icon on the left, typography and Byte-Labs badge on the right.
    """
    img = Image.new("RGBA", (width, height), BG_DARK)
    draw = ImageDraw.Draw(img)

    # Ambient background circuit elements
    for y in range(0, height, 40):
        draw.line([(0, y), (width, y)], fill=(20, 28, 48, 60), width=1)
    for x in range(0, width, 40):
        draw.line([(x, 0), (x, height)], fill=(20, 28, 48, 60), width=1)

    # Subtle cyan & amber energy gradients in corners
    glow = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow)
    gdraw.ellipse([width - 300, -100, width + 200, 400], fill=(0, 229, 255, 30))
    gdraw.ellipse([-100, height - 300, 400, height + 200], fill=(255, 107, 0, 40))
    glow = glow.filter(ImageFilter.GaussianBlur(50))
    img = Image.alpha_composite(img, glow)
    draw = ImageDraw.Draw(img)

    # Render icon on the left
    icon_size = 460
    icon = create_supersampled_icon(size=1024, unplated=False)
    icon_resized = icon.resize((icon_size, icon_size), Image.Resampling.LANCZOS)
    img.paste(icon_resized, (70, 70), icon_resized)

    # Text area on the right
    # Try finding system fonts, or draw crisp stylized geometric vectors for R.I.D.E
    # Draw "R . I . D . E"
    text_x = 580
    text_y = 150

    try:
        font_title = ImageFont.truetype("segoeui.ttf", 108)
        font_sub = ImageFont.truetype("segoeui.ttf", 36)
        font_badge = ImageFont.truetype("segoeuib.ttf", 26)
    except Exception:
        font_title = ImageFont.load_default()
        font_sub = ImageFont.load_default()
        font_badge = ImageFont.load_default()

    # Brand Title: "R.I.D.E"
    draw.text((text_x, text_y), "R . I . D . E", fill=(255, 255, 255, 255), font=font_title)

    # Accent underline gradient bar
    bar_y = text_y + 135
    for bx in range(text_x, text_x + 520):
        t = (bx - text_x) / 520.0
        r = int(255 - t * 255)
        g = int(107 + t * 122)
        b = int(0 + t * 255)
        draw.line([(bx, bar_y), (bx, bar_y + 5)], fill=(r, g, b, 255))

    # Subtitle: "Rust Intelligent Development Environment"
    draw.text((text_x, bar_y + 25), "Rust Intelligent Development Environment", fill=TEXT_MUTED, font=font_sub)

    # Byte-Labs Badge Tag
    badge_x = text_x
    badge_y = bar_y + 90
    draw.rounded_rectangle([badge_x, badge_y, badge_x + 220, badge_y + 44], radius=8, fill=(15, 23, 42, 255), outline=(0, 229, 255, 180), width=2)
    # Small pulsing dot
    draw.ellipse([badge_x + 14, badge_y + 16, badge_x + 26, badge_y + 28], fill=CYAN_GLOW)
    draw.text((badge_x + 36, badge_y + 8), "BY BYTE-LABS", fill=(224, 242, 254, 255), font=font_badge)

    # On-Device MoE pill
    moe_x = badge_x + 240
    draw.rounded_rectangle([moe_x, badge_y, moe_x + 200, badge_y + 44], radius=8, fill=(30, 20, 10, 255), outline=(255, 107, 0, 180), width=2)
    draw.ellipse([moe_x + 14, badge_y + 16, moe_x + 26, badge_y + 28], fill=RUST_ORANGE)
    draw.text((moe_x + 36, badge_y + 8), "LOCAL AI MoE", fill=(254, 243, 199, 255), font=font_badge)

    return img


def create_splash_screen(width=2480, height=1200):
    """
    Renders master SplashScreen (2480x1200) for scaling down cleanly.
    Deep obsidian backdrop with center logo, radiant glow, and pristine typography.
    """
    img = Image.new("RGBA", (width, height), BG_DARK)
    draw = ImageDraw.Draw(img)

    # Cosmic neural network background particles & subtle lines
    glow = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow)
    cx, cy = width // 2, height // 2 - 80
    for r in range(500, 0, -25):
        alpha = int(35 * (1.0 - r / 500))
        gdraw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(255, 107, 0, alpha))
    for r in range(300, 0, -20):
        alpha = int(40 * (1.0 - r / 300))
        gdraw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(0, 229, 255, alpha))
    glow = glow.filter(ImageFilter.GaussianBlur(60))
    img = Image.alpha_composite(img, glow)
    draw = ImageDraw.Draw(img)

    # Center Icon
    icon_size = 500
    icon = create_supersampled_icon(size=1024, unplated=False)
    icon_resized = icon.resize((icon_size, icon_size), Image.Resampling.LANCZOS)
    img.paste(icon_resized, (cx - icon_size // 2, cy - icon_size // 2 - 30), icon_resized)

    # Typography
    try:
        font_title = ImageFont.truetype("segoeui.ttf", 96)
        font_sub = ImageFont.truetype("segoeui.ttf", 36)
        font_footer = ImageFont.truetype("segoeuib.ttf", 28)
    except Exception:
        font_title = ImageFont.load_default()
        font_sub = ImageFont.load_default()
        font_footer = ImageFont.load_default()

    title_text = "R . I . D . E"
    sub_text = "Rust Intelligent Development Environment"
    footer_text = "ENGINEERED BY BYTE-LABS  •  ZERO-CLOUD LOCAL AI"

    # Measure / Center title
    bbox_title = draw.textbbox((0, 0), title_text, font=font_title)
    w_title = bbox_title[2] - bbox_title[0]
    draw.text((cx - w_title // 2, cy + 270), title_text, fill=(255, 255, 255, 255), font=font_title)

    bbox_sub = draw.textbbox((0, 0), sub_text, font=font_sub)
    w_sub = bbox_sub[2] - bbox_sub[0]
    draw.text((cx - w_sub // 2, cy + 390), sub_text, fill=TEXT_MUTED, font=font_sub)

    bbox_foot = draw.textbbox((0, 0), footer_text, font=font_footer)
    w_foot = bbox_foot[2] - bbox_foot[0]
    draw.text((cx - w_foot // 2, cy + 470), footer_text, fill=(0, 229, 255, 220), font=font_footer)

    return img


def create_hero_artwork(width=2400, height=1200):
    """
    Renders high-res Microsoft Store promotional artwork (2400x1200).
    Ideal for Partner Center Store Listing Header / Hero image.
    """
    img = Image.new("RGBA", (width, height), (8, 11, 18, 255))
    draw = ImageDraw.Draw(img)

    # High-tech matrix grid
    for x in range(0, width, 50):
        draw.line([(x, 0), (x, height)], fill=(18, 26, 42, 60), width=1)
    for y in range(0, height, 50):
        draw.line([(0, y), (width, y)], fill=(18, 26, 42, 60), width=1)

    # Dual atmospheric glows (Rust Orange left, Quantum Blue right)
    glow = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow)
    gdraw.ellipse([-200, -200, 1000, 1000], fill=(255, 107, 0, 50))
    gdraw.ellipse([width - 800, 200, width + 400, height + 400], fill=(0, 229, 255, 45))
    glow = glow.filter(ImageFilter.GaussianBlur(90))
    img = Image.alpha_composite(img, glow)
    draw = ImageDraw.Draw(img)

    # Hero Icon Left
    icon_size = 680
    icon = create_supersampled_icon(size=1024, unplated=False)
    icon_resized = icon.resize((icon_size, icon_size), Image.Resampling.LANCZOS)
    img.paste(icon_resized, (150, 260), icon_resized)

    # Content Right
    content_x = 930
    content_y = 280

    try:
        f_super = ImageFont.truetype("segoeuib.ttf", 30)
        f_brand = ImageFont.truetype("segoeuib.ttf", 112)
        f_pitch = ImageFont.truetype("segoeui.ttf", 40)
        f_feat = ImageFont.truetype("segoeui.ttf", 32)
    except Exception:
        f_super = ImageFont.load_default()
        f_brand = ImageFont.load_default()
        f_pitch = ImageFont.load_default()
        f_feat = ImageFont.load_default()

    # Eyebrow tag
    draw.rounded_rectangle([content_x, content_y, content_x + 280, content_y + 44], radius=6, fill=(15, 23, 42, 255), outline=(0, 229, 255, 200), width=2)
    draw.text((content_x + 20, content_y + 6), "BYTE-LABS PRESENTS", fill=(0, 229, 255, 255), font=f_super)

    # Brand Title
    draw.text((content_x, content_y + 65), "R . I . D . E", fill=(255, 255, 255, 255), font=f_brand)

    # Accent rule
    rule_y = content_y + 215
    for rx in range(content_x, content_x + 720):
        t = (rx - content_x) / 720.0
        r = int(255 - t * 255)
        g = int(107 + t * 122)
        b = int(0 + t * 255)
        draw.line([(rx, rule_y), (rx, rule_y + 6)], fill=(r, g, b, 255))

    # Tagline
    draw.text((content_x, rule_y + 30), "The Sovereign Rust Intelligent Development Environment", fill=(226, 232, 240, 255), font=f_pitch)

    # Feature Bullet Pillars
    features = [
        ("✦", "On-Device MoE Local Intelligence (Ollama / Llama.cpp / ONNX)"),
        ("✦", "Zero Cloud Telemetry — 100% Private Sovereign Code Security"),
        ("✦", "Sub-Millisecond Multi-Tier Workspace Indexing & Discovery"),
        ("✦", "Hyper-Responsive Native Windows Desktop Engine with Tauri 2.0")
    ]
    feat_y = rule_y + 120
    for bullet, text in features:
        draw.text((content_x, feat_y), bullet, fill=(255, 107, 0, 255), font=f_feat)
        draw.text((content_x + 40, feat_y), text, fill=TEXT_MUTED, font=f_feat)
        feat_y += 60

    return img


def generate_vector_svg(output_path):
    """
    Writes the official vector SVG master logo.
    """
    svg_content = """<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
  <defs>
    <!-- Background Gradient -->
    <radialGradient id="bgGlow" cx="50%" cy="50%" r="65%">
      <stop offset="0%" stop-color="#141C30"/>
      <stop offset="60%" stop-color="#0A0D16"/>
      <stop offset="100%" stop-color="#05070B"/>
    </radialGradient>

    <!-- Rust Flame Gradient -->
    <linearGradient id="rustGradient" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#FFAE00"/>
      <stop offset="35%" stop-color="#FF6B00"/>
      <stop offset="100%" stop-color="#D94100"/>
    </linearGradient>

    <!-- Cyber Cyan Gradient -->
    <linearGradient id="cyanGradient" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#E0F2FE"/>
      <stop offset="30%" stop-color="#38BDF8"/>
      <stop offset="70%" stop-color="#0284C7"/>
      <stop offset="100%" stop-color="#0E7490"/>
    </linearGradient>

    <!-- Leg Thrust Gradient -->
    <linearGradient id="thrustGradient" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#FF6B00"/>
      <stop offset="50%" stop-color="#FF9A22"/>
      <stop offset="100%" stop-color="#EF4444"/>
    </linearGradient>

    <!-- Border Glow Filter -->
    <filter id="neonGlow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="14" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>

    <filter id="softShadow" x="-10%" y="-10%" width="120%" height="120%">
      <feDropShadow dx="0" dy="16" stdDeviation="24" flood-color="#000000" flood-opacity="0.7"/>
    </filter>
  </defs>

  <!-- Ambient Glow -->
  <rect x="30" y="30" width="964" height="964" rx="210" fill="#FF6B00" opacity="0.12" filter="url(#neonGlow)"/>

  <!-- Squircle Base -->
  <rect x="40" y="40" width="944" height="944" rx="200" fill="url(#bgGlow)" stroke="#2D3C5A" stroke-width="5" filter="url(#softShadow)"/>

  <!-- Radial Core Glow -->
  <circle cx="512" cy="512" r="320" fill="#FF6B00" opacity="0.08"/>
  <circle cx="530" cy="430" r="180" fill="#00E5FF" opacity="0.12"/>

  <!-- Main R Glyph Group -->
  <g filter="url(#softShadow)">
    <!-- Rail 1: Outer Vertical Pillar -->
    <rect x="220" y="210" width="70" height="600" rx="12" fill="url(#rustGradient)"/>

    <!-- Rail 2: Inner Vertical Track -->
    <rect x="315" y="210" width="50" height="600" rx="8" fill="url(#rustGradient)" opacity="0.9"/>

    <!-- Byte-Labs Circuit Accents on Left Rail -->
    <g fill="#00E5FF">
      <rect x="205" y="315" width="20" height="10" rx="4"/>
      <circle cx="190" cy="320" r="6"/>
      <line x1="196" y1="320" x2="205" y2="320" stroke="#00E5FF" stroke-width="3"/>

      <rect x="205" y="425" width="20" height="10" rx="4"/>
      <circle cx="190" cy="430" r="6"/>
      <line x1="196" y1="430" x2="205" y2="430" stroke="#00E5FF" stroke-width="3"/>

      <rect x="205" y="535" width="20" height="10" rx="4"/>
      <circle cx="190" cy="540" r="6"/>
      <line x1="196" y1="540" x2="205" y2="540" stroke="#00E5FF" stroke-width="3"/>

      <rect x="205" y="645" width="20" height="10" rx="4"/>
      <circle cx="190" cy="650" r="6"/>
      <line x1="196" y1="650" x2="205" y2="650" stroke="#00E5FF" stroke-width="3"/>
    </g>

    <!-- Upper Bowl (Aerodynamic R Loop) -->
    <path d="M 365 210 L 620 210 Q 790 210 790 390 Q 790 570 620 570 L 365 570 L 365 485 L 600 485 Q 695 485 695 390 Q 695 295 600 295 L 365 295 Z" fill="url(#rustGradient)"/>

    <!-- Dynamic Forward Thruster Leg -->
    <path d="M 480 520 L 600 520 L 805 785 Q 825 810 790 810 L 710 810 L 560 610 L 460 610 Z" fill="url(#thrustGradient)"/>
    <!-- Speed Rail on Leg -->
    <path d="M 580 530 L 610 530 L 825 810 L 795 810 Z" fill="#FFD700" opacity="0.85"/>
  </g>

  <!-- Neural MoE Diamond Core (AI Engine inside Loop) -->
  <g transform="translate(520, 390)">
    <!-- Core Glow -->
    <circle cx="0" cy="0" r="85" fill="#00E5FF" opacity="0.25" filter="url(#neonGlow)"/>

    <!-- Synaptic Beam -->
    <line x1="-140" y1="0" x2="140" y2="0" stroke="#00E5FF" stroke-width="3" opacity="0.6"/>
    <circle cx="-140" cy="0" r="5" fill="#00E5FF"/>
    <circle cx="140" cy="0" r="5" fill="#00E5FF"/>

    <!-- 3D Diamond Facets -->
    <!-- Top-Left Facet -->
    <polygon points="0,-75 -55,0 0,0" fill="#38BDF8"/>
    <!-- Top-Right Facet -->
    <polygon points="0,-75 55,0 0,0" fill="#7DD3FC"/>
    <!-- Bottom-Left Facet -->
    <polygon points="0,75 -55,0 0,0" fill="#0284C7"/>
    <!-- Bottom-Right Facet -->
    <polygon points="0,75 55,0 0,0" fill="#0EA5E9"/>

    <!-- Center White Hot Core -->
    <polygon points="0,-24 16,0 0,24 -16,0" fill="#FFFFFF"/>
  </g>
</svg>
"""
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(svg_content)
    print(f"Generated Vector SVG: {output_path}")


def main():
    print("=" * 70)
    print("  R.I.D.E — Generating Master Brand Assets & Microsoft Store Kit")
    print("  Publisher: Byte-Labs | Identity: Byte-Labs.R.I.D.E")
    print("=" * 70)

    # 1. Generate Master Vector SVG
    svg_path = os.path.join(ASSETS_DIR, "logo.svg")
    generate_vector_svg(svg_path)

    # 2. Render Master Super-sampled Images
    print("Rendering Master 1024x1024 Plated & Unplated Icons...")
    master_plated = create_supersampled_icon(size=1024, unplated=False)
    master_unplated = create_supersampled_icon(size=1024, unplated=True)
    master_light_unplated = master_unplated.copy()  # For light themes, unplated icon has rich colors

    master_wide = create_wide_tile(width=1240, height=600)
    master_splash = create_splash_screen(width=2480, height=1200)
    master_hero = create_hero_artwork(width=2400, height=1200)

    # Save Master Previews
    master_plated.save(os.path.join(ASSETS_DIR, "logo-1024.png"), "PNG")
    master_unplated.save(os.path.join(ASSETS_DIR, "logo-unplated-1024.png"), "PNG")
    master_hero.save(os.path.join(ASSETS_DIR, "HeroArt2400x1200.png"), "PNG")

    # Partner Center Listing Promotional Images
    master_hero.resize((1920, 1080), Image.Resampling.LANCZOS).save(
        os.path.join(ASSETS_DIR, "HeroArt1920x1080.png"), "PNG"
    )
    master_hero.resize((1280, 720), Image.Resampling.LANCZOS).save(
        os.path.join(ASSETS_DIR, "HeroArt1280x720.png"), "PNG"
    )
    master_plated.resize((300, 300), Image.Resampling.LANCZOS).save(
        os.path.join(ASSETS_DIR, "StoreIcon300x300.png"), "PNG"
    )

    # -----------------------------------------------------------------------
    # MICROSOFT STORE REQUIRED SIZES & NAMING CONVENTIONS
    # -----------------------------------------------------------------------
    print("\nGenerating Microsoft Store MSIX/AppX Manifest Asset Suite...")

    # StoreLogo (Standard size: 50x50, scales: 100, 125, 150, 200, 400)
    store_logo_scales = {
        "StoreLogo.png": (50, 50),
        "StoreLogo.scale-100.png": (50, 50),
        "StoreLogo.scale-125.png": (63, 63),
        "StoreLogo.scale-150.png": (75, 75),
        "StoreLogo.scale-200.png": (100, 100),
        "StoreLogo.scale-400.png": (200, 200),
    }
    for filename, (w, h) in store_logo_scales.items():
        master_plated.resize((w, h), Image.Resampling.LANCZOS).save(os.path.join(ASSETS_DIR, filename), "PNG")
        print(f"  + {filename} ({w}x{h})")

    # Square44x44Logo (App list, taskbar, Alt-Tab)
    # Scales:
    sq44_scales = {
        "Square44x44Logo.png": (44, 44),
        "Square44x44Logo.scale-100.png": (44, 44),
        "Square44x44Logo.scale-125.png": (55, 55),
        "Square44x44Logo.scale-150.png": (66, 66),
        "Square44x44Logo.scale-200.png": (88, 88),
        "Square44x44Logo.scale-400.png": (176, 176),
    }
    for filename, (w, h) in sq44_scales.items():
        master_plated.resize((w, h), Image.Resampling.LANCZOS).save(os.path.join(ASSETS_DIR, filename), "PNG")
        print(f"  + {filename} ({w}x{h})")

    # TargetSize (Taskbar, Task Switcher, Jump Lists)
    for ts in [16, 24, 32, 48, 256]:
        # Plated
        master_plated.resize((ts, ts), Image.Resampling.LANCZOS).save(
            os.path.join(ASSETS_DIR, f"Square44x44Logo.targetsize-{ts}.png"), "PNG"
        )
        # Altform-unplated (Dark taskbar)
        master_unplated.resize((ts, ts), Image.Resampling.LANCZOS).save(
            os.path.join(ASSETS_DIR, f"Square44x44Logo.altform-unplated_targetsize-{ts}.png"), "PNG"
        )
        # Altform-lightunplated (Light taskbar)
        master_light_unplated.resize((ts, ts), Image.Resampling.LANCZOS).save(
            os.path.join(ASSETS_DIR, f"Square44x44Logo.altform-lightunplated_targetsize-{ts}.png"), "PNG"
        )
        print(f"  + Square44x44Logo targetsize-{ts} (plated/unplated/lightunplated)")

    # Square71x71Logo (Small tile)
    sq71_scales = {
        "Square71x71Logo.png": (71, 71),
        "Square71x71Logo.scale-100.png": (71, 71),
        "Square71x71Logo.scale-125.png": (89, 89),
        "Square71x71Logo.scale-150.png": (107, 107),
        "Square71x71Logo.scale-200.png": (142, 142),
        "Square71x71Logo.scale-400.png": (284, 284),
    }
    for filename, (w, h) in sq71_scales.items():
        master_plated.resize((w, h), Image.Resampling.LANCZOS).save(os.path.join(ASSETS_DIR, filename), "PNG")
        print(f"  + {filename} ({w}x{h})")

    # Square150x150Logo (Medium tile)
    sq150_scales = {
        "Square150x150Logo.png": (150, 150),
        "Square150x150Logo.scale-100.png": (150, 150),
        "Square150x150Logo.scale-125.png": (188, 188),
        "Square150x150Logo.scale-150.png": (225, 225),
        "Square150x150Logo.scale-200.png": (300, 300),
        "Square150x150Logo.scale-400.png": (600, 600),
    }
    for filename, (w, h) in sq150_scales.items():
        master_plated.resize((w, h), Image.Resampling.LANCZOS).save(os.path.join(ASSETS_DIR, filename), "PNG")
        print(f"  + {filename} ({w}x{h})")

    # Wide310x150Logo (Wide tile)
    wide_scales = {
        "Wide310x150Logo.png": (310, 150),
        "Wide310x150Logo.scale-100.png": (310, 150),
        "Wide310x150Logo.scale-125.png": (388, 188),
        "Wide310x150Logo.scale-150.png": (465, 225),
        "Wide310x150Logo.scale-200.png": (620, 300),
        "Wide310x150Logo.scale-400.png": (1240, 600),
    }
    for filename, (w, h) in wide_scales.items():
        master_wide.resize((w, h), Image.Resampling.LANCZOS).save(os.path.join(ASSETS_DIR, filename), "PNG")
        print(f"  + {filename} ({w}x{h})")

    # Square310x310Logo (Large tile)
    sq310_scales = {
        "Square310x310Logo.png": (310, 310),
        "Square310x310Logo.scale-100.png": (310, 310),
        "Square310x310Logo.scale-125.png": (388, 388),
        "Square310x310Logo.scale-150.png": (465, 465),
        "Square310x310Logo.scale-200.png": (620, 620),
        "Square310x310Logo.scale-400.png": (1240, 1240),
    }
    for filename, (w, h) in sq310_scales.items():
        master_plated.resize((w, h), Image.Resampling.LANCZOS).save(os.path.join(ASSETS_DIR, filename), "PNG")
        print(f"  + {filename} ({w}x{h})")

    # SplashScreen (620x300 base, scales 100, 125, 150, 200, 400)
    splash_scales = {
        "SplashScreen.png": (620, 300),
        "SplashScreen.scale-100.png": (620, 300),
        "SplashScreen.scale-125.png": (775, 375),
        "SplashScreen.scale-150.png": (930, 450),
        "SplashScreen.scale-200.png": (1240, 600),
        "SplashScreen.scale-400.png": (2480, 1200),
    }
    for filename, (w, h) in splash_scales.items():
        master_splash.resize((w, h), Image.Resampling.LANCZOS).save(os.path.join(ASSETS_DIR, filename), "PNG")
        print(f"  + {filename} ({w}x{h})")

    # BadgeLogo (Lock screen badge: 24x24 base, scales)
    badge_scales = {
        "BadgeLogo.png": (24, 24),
        "BadgeLogo.scale-100.png": (24, 24),
        "BadgeLogo.scale-125.png": (30, 30),
        "BadgeLogo.scale-150.png": (36, 36),
        "BadgeLogo.scale-200.png": (48, 48),
        "BadgeLogo.scale-400.png": (96, 96),
    }
    for filename, (w, h) in badge_scales.items():
        master_unplated.resize((w, h), Image.Resampling.LANCZOS).save(os.path.join(ASSETS_DIR, filename), "PNG")
        print(f"  + {filename} ({w}x{h})")

    # 3. Create Windows Multi-Resolution .ICO (16, 24, 32, 48, 64, 128, 256)
    ico_path = os.path.join(ASSETS_DIR, "icon.ico")
    icon_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    ico_images = [master_plated.resize(s, Image.Resampling.LANCZOS) for s in icon_sizes]
    ico_images[0].save(ico_path, format="ICO", sizes=icon_sizes)
    print(f"\nGenerated Windows Multi-Resolution ICO: {ico_path}")

    # Also update crates/ride-desktop/icons/icon.ico so the desktop app uses the new logo
    desktop_ico_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "crates", "ride-desktop", "icons", "icon.ico"))
    if os.path.exists(os.path.dirname(desktop_ico_path)):
        ico_images[0].save(desktop_ico_path, format="ICO", sizes=icon_sizes)
        print(f"Updated Desktop Tauri Icon: {desktop_ico_path}")

    # Also copy 32x32 / 128x128 PNG to crates/ride-desktop/icons/ for Tauri icon packaging
    for s in [32, 128]:
        png_target = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "crates", "ride-desktop", "icons", f"{s}x{s}.png"))
        master_plated.resize((s, s), Image.Resampling.LANCZOS).save(png_target, "PNG")
        print(f"Updated Desktop Tauri Icon: {png_target}")

    print("\n[SUCCESS] All Microsoft Store visual assets successfully generated!")


if __name__ == "__main__":
    main()
