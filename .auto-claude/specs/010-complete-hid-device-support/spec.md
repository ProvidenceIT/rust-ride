# Complete HID Device Support

Finish HID device enumeration and control for Stream Deck and USB button controllers. Enable configurable button mappings for workout control without touching keyboard/mouse.

## Rationale
HID device support enables hands-free training control, improving the workout experience especially during intense intervals when reaching for controls is impractical.

## User Stories
- As a cyclist with a Stream Deck, I want to control my workout with physical buttons so that I don't need to reach for my computer
- As a user, I want to customize button mappings so that they match my preferred workflow

## Acceptance Criteria
- [ ] Stream Deck devices are detected and connected
- [ ] Button presses trigger configured actions (pause, lap, skip interval)
- [ ] Button mappings are user-configurable
- [ ] Works with common USB button controllers
- [ ] Device reconnects automatically if unplugged/replugged
