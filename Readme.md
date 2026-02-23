
# Allocator (Rust, no_std)

This library provides a compact physical + virtual memory allocator designed for early‑stage kernel development in `no_std` Rust environments.
It is fully self‑contained and uses neither dynamic allocation nor the standard library.

## ✨ Features

### 🧱 Physical Memory Allocator
- Uses a bitmap `[u8; F]` to track the status of frames.
- Implements a **first‑fit** strategy with **offset fast‑path**: allocations at the end are O(1) when space is available.
- Falls back to **hole search** when contiguous memory at the offset is not free.
- Supports alloc/free of contiguous physical frames.
- Can detect fragmentation and realign the offset upon deallocation.

### 🧬 Virtual Memory Layer (Soft‑MMU)
A minimal virtual layer designed without a real MMU.
It creates **virtual segments** local to each allocation.

Each allocation returns a `VirtualAddr` struct:
```rust
VirtualAddr { id: u16, pos: 0, ord: 0 }
```
Where:
- `id` = process identifier
- `pos = 0` = base of the virtual segment (every allocation starts at a local base)
- `ord` = index of the page inside the segment

The allocator:
- Computes how many frames are needed for a process.
- Attempts to allocate a contiguous physical block.
- If not possible, uses **adaptive fragmentation handling** with a recursive fallback, returning progressively smaller fragments.
- Maps each fragment into an `IdEntry` describing a single virtual page.

### 📦 IdEntry
Represents a binding between a virtual page and its physical frame:
```rust
IdEntry {
    addr: VirtualAddr,   // virtual identity
    frame: u16,          // physical frame number
    ptr: *mut u8         // pointer to real memory inside bytes[N]
}
```

## 📌 What This Library Is Not (Yet)
- ❌ No virtual deallocation (`free_virt`) implemented yet.
- ❌ No virtual page lookup helpers (e.g., virtual read/write).
- ❌ No global virtual address space — each `VirtualAddr` is local to one allocation.
- ❌ No real MMU integration.

These features can be added on top once the virtual mapping logic is finished.

## 🧪 Configuration Parameters
The allocator is generic over three compile‑time constants:
```rust
Allocator<N, S, F>
```
Where:
- `N`: size of the backing memory in bytes
- `S`: max number of virtual IdEntry slots
- `F`: number of physical frames

Constraints:
- `N % F == 0`
- `N > F` (frame size >= 1 byte)
- `N` and `F` must be powers of two

## 🛠 Public API

### Constructor
```rust
Allocator::new() -> Option<Self>
```
Validates parameters and constructs the allocator.

### Virtual Allocation
```rust
alloc(&mut self, process: u16, need_bytes: usize)
    -> (Option<VirtualAddr>, AllocResult)
```
Returns:
- `Some(VirtualAddr)` if allocation succeeds
- `AllocResult::NotEnoughMemory` otherwise

### Physical Free
```rust
fn free_phys(&mut self, slice: &mut [u8]) -> bool
```
Used internally and available for tests.

## 🔒 Safety Notes
- This library uses raw pointers (`*mut u8`) but always inside controlled internal logic.
- No &mut aliasing escapes to the user.
- All returned `VirtualAddr` are symbolic and must not be interpreted as raw pointers.

## 📜 License
MIT or Apache‑2.0 (same as Rust ecosystem conventions).

## 🚧 Status
- Stable physical allocator
- Experimental virtual allocator
- Virtual free + VM access functions coming next

