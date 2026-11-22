# Microsoft Store Submission Guide — R.I.D.E

**Product**: R.I.D.E — Rust Intelligent Development Environment  
**Publisher**: Byte-Labs  
**Store ID**: `9PP3JK8T30DW`  
**Package Family Name (PFN)**: `Byte-Labs.R.I.D.E_gnbqk611y9p3y`  
**Package Version**: `2.0.0.0`  

---

## 1. Registered Partner Center Identity

This package is configured to match the Microsoft Partner Center app identity:

| Identity Parameter | Configured Value |
| :--- | :--- |
| **Package/Identity/Name** | `Byte-Labs.R.I.D.E` |
| **Package/Identity/Publisher** | `CN=6CDD7E10-2EAB-4D1D-B6CA-5D8F278050DF` |
| **Package/Properties/PublisherDisplayName** | `Byte-Labs` |
| **Package Family Name (PFN)** | `Byte-Labs.R.I.D.E_gnbqk611y9p3y` |
| **Package SID** | `S-1-15-2-2973230389-2040763780-3832071188-716803835-3111349494-3869605249-1870189185` |
| **Store ID** | `9PP3JK8T30DW` |
| **Target Architecture** | `x64` |
| **Minimum Windows Version** | Windows 10 version 1809 (Build 17763) |

---

## 2. Quick Packaging Commands

From PowerShell in the repository root:

### Build and Package for Microsoft Store
```powershell
# From the msstore directory:
cd msstore
.\package_msix.ps1 -BuildRelease
```
This produces:
- `msstore/dist/Byte-Labs.R.I.D.E_2.0.0.0_x64.msix` (MSIX Package)
- `msstore/dist/Byte-Labs.R.I.D.E_2.0.0.0_x64.msixupload` (Direct Partner Center upload bundle)

### Build and Sign for Local Sideload Testing
To test install the package directly on your local Windows PC before uploading:
```powershell
.\package_msix.ps1 -BuildRelease -SignForLocalTesting
```
Then double-click `Byte-Labs.R.I.D.E_2.0.0.0_x64.msix` to install.

---

## 3. Microsoft Partner Center Upload Steps

1. Log in to [Microsoft Partner Center](https://partner.microsoft.com/dashboard).
2. Under **Apps and Games**, locate your registered app with Store ID `9PP3JK8T30DW` (**Byte-Labs.R.I.D.E**).
3. Click **Start your submission** (or **Update**).
4. Navigate to the **Packages** page:
   - Drag and drop `msstore/dist/Byte-Labs.R.I.D.E_2.0.0.0_x64.msixupload` into the upload box.
   - Partner Center will validate package name (`Byte-Labs.R.I.D.E`), publisher identity (`CN=6CDD7E10-2EAB-4D1D-B6CA-5D8F278050DF`), version (`2.0.0.0`), and target architectures (`x64`).
5. Proceed to **Store Listings** and enter the metadata detailed below.

---

## 4. Store Listing Information (Copy-Paste Ready)

### Product Name
```
R.I.D.E — Rust Intelligent Development Environment
```

### Short Description (Under 100 characters)
```
High-performance, sovereign Rust IDE with on-device MoE neural intelligence and zero cloud telemetry.
```

### Full Description
```
R.I.D.E (Rust Intelligent Development Environment) by Byte-Labs is a sovereign, next-generation code editor built from the ground up for privacy-conscious engineers and high-performance teams.

Key Highlights:

• Sovereign Local AI (MoE): Connects seamlessly to on-device inference engines including Ollama, ColibriMoE (NVMe memory-mapped streaming), LM Studio, and Llama.cpp. Your proprietary code never leaves your local hardware.

• Blazing Fast Native Performance: Engineered with Rust, Tokio, and Tauri 2.0. Near-zero memory overhead and instant startup times without sluggish web runtimes.

• Multi-Layout Adaptive Workspace: Switch effortlessly between three specialized development modes:
   - Editor (VS Code style): Full workspace tree, tabs, integrated diff reviewer, and terminal.
   - Agent (Claude Code style): Conversational pair-programmer with interactive diff proposals and one-click shell command approvals.
   - Zen (Codex style): Clean, distraction-free minimalist code canvas.

• Integrated Code Diff & Rollback: Real-time visual change tracking with syntax-aware unified and side-by-side diff previews. Apply, review, or rollback edits with a single click.

• Dynamic Skills System: Extensible autonomous agent capabilities for deep Rust optimization, Clean Code principles, systematic debugging, and API security auditing.

• Sub-Millisecond Workspace Discovery: Instant regex and fuzzy symbol search indexing across hundreds of thousands of lines of code.

Engineered with precision by Byte-Labs.
```

### Key Features (Up to 20 items)
1. `Local On-Device Mixture of Experts (MoE) Intelligence`
2. `Zero Cloud Telemetry — 100% Private & Sovereign Code Security`
3. `Three Fluid Layouts: Editor, Agent Pair-Programmer, and Zen Canvas`
4. `One-Click Visual Diff Inspection, Safe Stash, and Instant Rollback`
5. `Automatic Local Model Discovery (Ollama, LM Studio, Llama.cpp)`
6. `Native Windows Execution via Sub-millisecond Rust & Tauri Core`
7. `Built-in Extensible Skills Engine (Rust Expert, Clean Code, Security)`
8. `High-DPI Native Windows Visuals with Dark & Light Theme Support`

### Search Keywords (Up to 7 terms)
- `rust`
- `ide`
- `code editor`
- `ai code assistant`
- `moe`
- `local llm`
- `developer tools`

### Category
- **Primary Category**: `Developer Tools`
- **Subcategory**: `Development Software`

---

## 5. Visual Assets & Screenshots

All visual assets have been rendered to Microsoft Store specifications.

### Store Screenshots (1920x1080 PNG)
Located in `msstore/Screenshots/`:
1. `01_Code_Editor_Workspace.png` — Code editor with syntax highlighting, file tree, tabs, and status bar.
2. `02_Intelligent_MoE_Agent.png` — Sovereign AI pair-programming agent, thinking traces, and tool execution.
3. `03_Visual_Diff_Code_Review.png` — Syntax-aware diff inspector with one-click apply and rollback.
4. `04_Skills_Manager_Local_AI.png` — Active Skills manager (rust-expert, clean-code, security) and local engines.
5. `05_Zen_Codex_Focus_Mode.png` — Minimalist distraction-free code canvas.

### Promotional Artwork & Hero Images
Located in `msstore/Assets/`:
- **App Icon (300x300)**: `msstore/Assets/StoreIcon300x300.png`
- **Hero Artwork (2400x1200)**: `msstore/Assets/HeroArt2400x1200.png`
- **Hero Artwork (1920x1080)**: `msstore/Assets/HeroArt1920x1080.png`
- **Hero Artwork (1280x720)**: `msstore/Assets/HeroArt1280x720.png`
- **Master Vector SVG**: `msstore/Assets/logo.svg`

---

## 6. Store Reviewer Certification Notes

When prompted by Microsoft Partner Center regarding the restricted capability `<rescap:Capability Name="runFullTrust"/>`:

### Justification Statement for Reviewers:
> "R.I.D.E is a developer code editor and integrated development environment (IDE). It requires runFullTrust to execute developer tools, compile code via cargo and rustc, launch local command-line processes in the integrated terminal, manage local file systems, and communicate with local inference runtimes (such as Ollama or Llama.cpp) on localhost loopback network sockets."

---

## 7. Submission Checklist

- [x] Package identity strictly verified against Partner Center (`Byte-Labs.R.I.D.E`)
- [x] Publisher CN verified (`CN=6CDD7E10-2EAB-4D1D-B6CA-5D8F278050DF`)
- [x] Package Family Name confirmed (`Byte-Labs.R.I.D.E_gnbqk611y9p3y`)
- [x] All 5 high-resolution 1920x1080 screenshots captured and organized
- [x] StoreLogo and tile asset scales (100, 125, 150, 200, 400) generated
- [x] MSIX bundle automation tested and validated with `makeappx.exe`
- [x] FullTrust capability justification documented
- [x] Privacy policy and zero cloud telemetry statement included
