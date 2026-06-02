# Soul Symphony Ling — โปรแกรมเต็ม (ไทย)

## โครงสร้างโปรเจ็กต์

```
J:\soul-symphony-ling\
│
├── main.ling                         # จุดเข้า: ling run main.ling
├── Cargo.toml                        # แฟ้มกำหนดค่า Cargo (Rust)
├── README_TH.md                      # ไฟล์นี้
│
├── src/
│   └── main.rs                       # ตัวโหลด Rust (ลำดับการโหลดไฟล์)
│
├── เกม/                              # โมดูลเกมหลัก (ทั้งหมด ling-lang)
│   ├── สี.ling                       # ตัวช่วยสี — sine-wave, psychedelic, title
│   ├── รูปทรง.ling                   # รูปทรง 3D — merkaba, beach ball, ground sphere
│   ├── เสียง.ling                    # ชุดเสียง 4D — 5 แดน
│   ├── ชื่อเรื่อง.ling               # หน้าชื่อเรื่อง + glyph rings
│   ├── เล่น.ling                     # overlay realm-specific
│   └── หลัก.ling                     # ลูปเกม + physics + input (หลัก)
│
├── เสียง/                            # ไฟล์เสียง (WAV/OGG)
│   ├── ball-jump.wav
│   ├── ball_roll.wav
│   ├── ball_roll2.wav
│   ├── boing00.wav ... boing04.wav
│   ├── correct_guess.wav
│   ├── hover.wav
│   ├── incorrect_guess.wav
│   ├── pickup.wav
│   ├── portal_loop.wav ... portal_loop06.wav
│   ├── soul-symphony.wav
│   ├── start-dialog.wav
│   └── warp.wav
│
├── เพลง/                             # ไฟล์เพลง (OGG/FLAC)
│   ├── Ambience00.ogg
│   ├── Celestial.ogg
│   ├── Excellent.ogg
│   ├── Flag.ogg
│   ├── FlagTracker.ogg
│   ├── HallofSunrise.ogg
│   ├── House of White.flp             (Fruity Loops Project)
│   ├── NightDrive.ogg
│   ├── SpaceField.ogg
│   ├── SpiritsMarch.ogg
│   ├── StarlightVocals.ogg
│   ├── SuperSignal.ogg
│   ├── TheGreatJourney.ogg
│   ├── TheSpiritsTwo.ogg
│   ├── The_Spirit_Flag_Instrumental.ogg
│   ├── The_Spirits.ogg
│   ├── Through_my_Heart_Instrumental.ogg
│   ├── TitleScreen.ogg
│   ├── Today2.ogg
│   ├── Trich.ogg
│   ├── WalkThePath.ogg
│   ├── Whisper.ogg
│   ├── WishingWell.ogg
│   ├── Womper.ogg
│   └── YouMightBeRight.ogg
│
├── ภาพ/                              # ไฟล์ภาพ (PNG/SVG/JPG)
│   ├── Checker.png
│   ├── SoulSymphonyLogo.png
│   ├── SoulSymphonyLogo.svg
│   ├── ball_bottom.png
│   ├── ball_middle.png
│   ├── ball_top.png
│   ├── dialog_frame.png
│   ├── mandala00.png
│   ├── mandala01.png
│   ├── pattern00.png ... pattern06.png
│   ├── photo-1572756317709-fe9c15ced298.jfif
│   ├── spinning_symbol_00.svg
│   ├── tag00.png
│   ├── tag01.png
│   ├── tag02.png
│   ├── textwall.png
│   ├── tile00.png
│   ├── tile01.jpg
│   ├── tile01.png
│   └── tile02.png
│
├── ฟอนต์/                            # ฟอนต์ (OTF/G2N)
│   ├── ข้อความ/                      # ข้อความสาธารณะ (Alstoria, Circus, Copic, ...)
│   │   ├── Alstoria.otf
│   │   ├── Circus.g2n
│   │   ├── Circus.otf
│   │   ├── Copic.otf
│   │   ├── Dayton.otf
│   │   ├── Empire.otf
│   │   ├── Festival.otf
│   │   ├── Mallika.otf
│   │   ├── Storybook.otf
│   │   ├── Xenon.g2n
│   │   └── Xenon.otf
│   ├── จักร/                         # จักร (Daemon variants, Chesilin)
│   │   ├── Chesilin.otf
│   │   ├── Daemon.g2n
│   │   └── Daemon.otf
│   ├── สัญลักษณ์/                    # สัญลักษณ์พิเศษ (Gender, Para, Snowflakes)
│   │   ├── crossbats.otf
│   │   ├── Gender.otf
│   │   ├── Para.otf
│   │   ├── Snowflakes.otf
│   │   └── Unknown.otf
│   ├── อื่นๆ/                        # ตัวแปร Daemon อื่น
│   │   ├── Daemon_Alternative.g2n
│   │   ├── Daemon_Alternative.otf
│   │   ├── Daemon_Full_Working.otf
│   │   └── Daemon_Robust.otf
│   └── โลก/                          # โลก (Sama)
│       └── Sama.otf
│
└── target/                           # Rust build output (ไม่ต้องสนใจ)
```

## วิธีรัน

### 1. ใช้ ling interpreter โดยตรง
```bash
# จากรูทโปรเจ็กต์:
ling run main.ling

# หรือรันเกมโมดูลตรงๆ:
ling run เกม/หลัก.ling
```

### 2. Compile ผ่าน Rust + Cargo
```bash
cargo run --release
```

## โครงสร้างโค้ด (เกม/)

| ไฟล์ | ชื่อที่แสดง | หน้าที่ |
|-----|-----------|--------|
| **สี.ling** | ตัวช่วยสี | สี sine-wave, psychedelic, title screen |
| **รูปทรง.ling** | รูปทรง 3D | เมอร์คาบา, ลูกบอล (beach ball), พื้นดิน (sphere) |
| **เสียง.ling** | เสียง 4D | 5 ชุดเสียง (ชื่อเรื่อง + 4 แดน) |
| **ชื่อเรื่อง.ling** | หน้าชื่อเรื่อง | glyph rings, logo, letter rain |
| **เล่น.ling** | เล่นเกม | โอรา, overlay ต่างๆ |
| **หลัก.ling** | หลัก | ลูปเกม, physics, input, state machine |

## ภาษา & เทคโนโลยี

- **ling-lang 2030**: ระบบภาษาจากต้นฉบับ (polyglot, ไทย + อังกฤษ)
- **Panda3D**: software rasterizer (minifb windowing)
- **ling-audio**: 4D spatial audio synthesis (w-dimension sub-oscillators)
- **ling-physics**: Poincaré ball (hyperbolic sphere) + rigid-body dynamics
- **ling-graphics**: vector texture primitives (vtex_rings, vtex_chakra, ...)

## ค่าคงที่เกม (เกม/หลัก.ling)

```
โลก:
  รัศมีลูกบอล = 1.0
  รัศมีทรงกลม = 100.0
  ศูนย์กลาง_Y = -100.0

ฟิสิกส์:
  แรงโน้มถ่วง = 18.0
  ความเร่ง = 55.0
  ความเร็วสูง = 12.0
  ความเร็วกระโดด = 10.5
  จำนวนกระโดด = 2
  การเด้ง = 0.65
  แรงเสียดทาน = 0.87

กล้อง:
  ระยะกล้อง = 15.0
  ความสูงกล้อง = 4.5
  ความไวเมาส์ = 0.0024
  ระยะZ = 2.8
```

## การควบคุม

```
WASD + เมาส์   — เลื่อนลูกบอล + หมุนกล้อง
Space          — กระโดด/กระโดดคู่
E              — สนทนากับจิตใจ
N              — เปลี่ยนแดน (4 แดน)
ESC            — ออก
```

## 4 แดน

1. **แดน ๑ — ความว่างเปล่า** (The Void)
   - พื้นสีเข้ม, ลายเกลียว
   
2. **แดน ๒ — มหาสมุทรจิต** (Astral Ocean)
   - พื้นสีน้ำเงิน, ลายไฮเพอร์โบลิก
   
3. **แดน ๓ — อาณาจักรคริสตัล** (Crystal Realm)
   - พื้นสีม่วง, ลายตาข่าย
   
4. **แดน ๔ — สรณะแห่งดวงวิญญาณ** (Soul Sanctuary)
   - พื้นสีทอง, เมอร์คาบา + ลายดอก

## ประวัติศาสตร์

- **ต้นฉบับ (Python)**: J:\soul-symphony\ (Panda3D)
- **ปัจจุบัน (ling-lang)**: J:\soul-symphony-ling\ (ling interpreter + Rust)
- **ภาษา**: 100% Thai (keywords, functions, variables, constants, messages)

---

**Soul Symphony Ling** — A spiritual 3-D adventure through the interdimensional realm
