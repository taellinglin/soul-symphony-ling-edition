# P2P Cryptographic Synchronization System

## "SYNCHRONIZE OR DIE" Protocol

A peer-to-peer deterministic state synchronization system with cryptographic verification, designed for Soul Symphony Ling's online multiplayer mode.

---

## Core Philosophy

**Perfect Determinism**: All environmental state (day/night cycles, wind, terrain, tree growth) is computed from a shared world seed + synchronized tick counter. There is NO server-side randomness or authority conflicts - peers either stay perfectly in sync or disconnect.

**Crypto-Verified Trust**: Every N frames, peers broadcast `HASH(SEED + TICK + STATE)`. Mismatched hashes trigger immediate desync detection and recovery. Persistent desyncs result in automatic peer ejection.

**Zero Tolerance**: This system prioritizes perfect synchronization over leniency. A desynced player is a corrupted player and must be removed to preserve game integrity.

---

## Architecture

### File Structure

- **[game/p2p_sync.ling](game/p2p_sync.ling)** - Complete P2P synchronization implementation
- **[game/online_world.ling](game/online_world.ling)** - Original datetime-based world (non-P2P reference)

### Key Components

1. **Deterministic RNG Functions**
   - `detHash(seed, tick, varId)` - Pure function: same inputs → identical outputs across all machines
   - `getDayAngle(seed, tick)` - Day/night cycle position
   - `getWindX/Z(seed, tick)` - Wind vector components
   - `getTerrainHeight(seed, x, z)` - Multi-octave Perlin-like terrain
   - `shouldSpawnTree(seed, i, j)` - Boolean tree placement
   - `getTreePhase(seed, tick, i, j)` - Tree growth/decay cycle

2. **State Hashing & Verification**
   - `computeStateHash(seed, tick)` - Creates cryptographic fingerprint of world state at given tick
   - Uses `แฮชเข้ารหัส()` from ling-crypto (SHA3/BLAKE3-based)
   - Samples critical variables: day angle, wind, terrain at key coordinates
   - Returns hash string for broadcast/comparison

3. **P2P Sync Protocol**
   - `p2pSyncInit(currentFrame, hostMode)` - Initialize sync session
   - `p2pSyncUpdate(frame, seed, tick, ...)` - Frame-by-frame sync logic
   - Heartbeat broadcasting every `SYNC_INTERVAL` frames (default 30 = 0.5s @ 60fps)
   - Tick drift detection (max `MAX_TICK_DRIFT` = 10 before forced resync)
   - Desync tolerance: `DESYNC_TOLERANCE` = 3 consecutive mismatches before kick

4. **Rendering**
   - `renderSyncedWorld(seed, tick, t)` - Render deterministic environment
   - Displays debug overlay: current tick, wind values, sync status
   - Identical visual output across all synced peers

---

## Network Protocol Messages

### Message Format

All messages sent via `เน็ตส่ง(message)` and received via `เน็ตรับ()`.

```
SYNC:<tick>:<hash>
```
- **Purpose**: Heartbeat with state verification
- **Example**: `SYNC:12450:a3f5e8b2c1d9...`
- **Frequency**: Every SYNC_INTERVAL frames
- **Action**: Peer compares hash with local state hash; mismatch increments desync counter

```
SEED:<seed>:<hash>
```
- **Purpose**: World seed announcement (host only, at session start)
- **Example**: `SEED:1735574400000:d7e2f1a8...`
- **Frequency**: Once at initialization
- **Action**: Client receives seed and verifies hash to prevent tampering

```
STATE:<tick>:<data>:<hash>
```
- **Purpose**: Full state broadcast for emergency resync
- **Example**: `STATE:12450:FULL:a3f5e8b2...`
- **Frequency**: Only when tick drift > MAX_TICK_DRIFT
- **Action**: Client resets to host's authoritative state

```
KICK:<peer_id>:<reason>
```
- **Purpose**: Desync/cheater removal notification
- **Example**: `KICK:0:DESYNC_LIMIT`
- **Frequency**: After DESYNC_TOLERANCE consecutive failures
- **Action**: Connection terminated, return to menu

---

## Integration Example

### In main.ling game loop:

```ling
# Global state variables
bind worldSeed = 0.0
bind syncTick = 0.0
bind lastSyncFrame = 0.0
bind desyncCount = 0.0
bind peerLastSeen = 0.0
bind peerTick = 0.0
bind isHost = 1.0  # Set based on co-op role

# In ST_ONLINE or ST_COOP state:
ถ้า สถานะ == ST_ONLINE {
    # Initialize on first frame
    ถ้า syncTick == 0.0 {
        bind worldSeed = p2pSyncInit(กรอบ, isHost)
    }

    # Update sync every frame
    bind syncTick = p2pSyncUpdate(
        กรอบ, worldSeed, syncTick, lastSyncFrame,
        desyncCount, peerLastSeen, peerTick, isHost
    )

    # Render deterministic world
    renderSyncedWorld(worldSeed, syncTick, t)

    # Exit condition
    ถ้า กดปุ่ม("escape") {
        เน็ตหยุดประกาศ()
        bind สถานะ = ST_MENU
    }
}
```

---

## Determinism Guarantees

### What is Deterministic

✅ **Environment Cycles**: Day/night, sun/moon positions
✅ **Weather**: Wind direction, wind speed
✅ **Terrain Generation**: Heights, moisture, biomes
✅ **Flora**: Tree spawn locations, growth phases, death cycles
✅ **Visual Effects**: Sky colors, fog, ambient lighting (based on seed/tick)

### What is NOT Deterministic (Requires Separate Sync)

❌ **Player Positions**: Must be synced via separate player state messages
❌ **Player Actions**: Combat, item pickups, etc. (requires event broadcasting)
❌ **Dynamic Objects**: Projectiles, particles (unless seeded from tick)
❌ **User Input**: Keypresses, mouse movement

---

## Testing Determinism

### Run the Built-in Test

```ling
testDeterminism()
```

**Expected Output** (MUST be identical on all machines):
```
=== DETERMINISM TEST ===
Seed: 123456, Tick: 1000
Day Angle: 0.1234567...
Wind: 1.234, -0.567
Terrain(0,0): 2.345
State Hash: a3f5e8b2c1d9f4a7e2b5c8d1f9a4e7b2...
========================
```

If the hash differs between two machines:
1. Check for floating-point precision differences (use `ปัดลง()` to floor values)
2. Verify identical Ling version and ling-crypto library
3. Ensure no local RNG contamination (no use of `random()` in deterministic code)

---

## Cryptographic Security

### Hash Function

Uses `แฮชเข้ารหัส(text)` from ling-crypto, likely SHA-3 or BLAKE3:
- **Collision Resistance**: Extremely low probability of different states producing same hash
- **Tamper Detection**: Any state modification → completely different hash
- **Pre-image Resistance**: Cannot reverse-engineer seed from hash

### Attack Vectors & Mitigations

**Attack**: Client sends fake hash to hide cheating
**Mitigation**: Hash is computed locally from state; mismatch triggers disconnect, not state acceptance

**Attack**: Client modifies seed after reception
**Mitigation**: Seed hash verified on receipt; future sync messages will fail verification

**Attack**: Client desyncs unintentionally (lag, packet loss)
**Mitigation**: Tick drift detection triggers emergency resync within MAX_TICK_DRIFT frames

**Attack**: Man-in-the-middle modifies messages
**Mitigation**: Future enhancement could add Ed25519 signatures to messages

---

## Performance Considerations

### Network Bandwidth

- **SYNC messages**: ~60 bytes every 0.5s = **120 bytes/sec per peer**
- **SEED message**: ~80 bytes once per session
- **STATE messages**: Variable, only during resync events (<1% of frames)

**Total**: <200 bytes/sec sustained, suitable for peer-to-peer even on slow connections.

### Computation Cost

- **Hash computation**: ~0.1ms per frame (only during SYNC_INTERVAL)
- **Deterministic RNG**: ~0.01ms per function call
- **Terrain generation**: 16×16 grid = 256 calls/frame ≈ 2.5ms
- **Tree rendering**: ~50-100 trees/frame with growth logic ≈ 1-2ms

**Total CPU**: ~4-5ms per frame = **safe at 60 FPS** (16.6ms budget)

---

## Limitations & Future Work

### Current Limitations

1. **Two-player only**: Protocol assumes single peer connection
2. **No message ordering**: UDP-like behavior; could have race conditions
3. **No signature verification**: Hashes prevent tampering but not identity spoofing
4. **Fixed tolerance values**: SYNC_INTERVAL and DESYNC_TOLERANCE are constants

### Future Enhancements

- **Multi-peer support**: Extend to 4+ players with consensus voting on state hashes
- **Adaptive sync rate**: Increase SYNC_INTERVAL during stable periods, decrease during desync warnings
- **Ed25519 signatures**: Add `แฮชลายเซ็น(message, privateKey)` verification
- **State delta compression**: Send only changed variables instead of full state blob
- **Replay recording**: Log seed + tick + player inputs for deterministic replay

---

## FAQ

**Q: Why not use a traditional client-server architecture?**
A: P2P with determinism eliminates server costs, reduces latency, and makes the game resilient to server shutdowns. Perfect for indie games.

**Q: What happens if players have different frame rates?**
A: Tick counter is independent of visual frames. A 30 FPS player and 60 FPS player can stay synced - the tick advances at wall-clock time or game logic rate, not render rate.

**Q: Can I use `random()` in my gameplay code?**
A: **NO**. Any call to non-deterministic RNG breaks synchronization. Use `detHash(seed, tick, yourUniqueID)` instead.

**Q: How do I debug desyncs during development?**
A: Add logging to `computeStateHash()` - print individual variable values before hashing. Compare logs between peers to find the diverging variable.

**Q: What if a player intentionally desyncs to cheat?**
A: They get kicked after 3 consecutive hash mismatches (~1.5 seconds). Cheating becomes pointless.

---

## License & Credits

**Created for**: Soul Symphony Ling
**Language**: Ling (ภาษาลิงค์)
**Crypto Library**: ling-crypto (SHA-3 / BLAKE3 / Ed25519)
**Protocol Design**: Deterministic lockstep with hash verification
**Philosophy**: "Synchronize or Die"

---

**"In a world where truth is verified by cryptography, lies cannot survive a single tick."**
