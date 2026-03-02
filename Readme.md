Allocator — A no_std Physical & Virtual Memory Allocator in Rust
This project provides a fully‑featured memory allocator designed for no_std environments.
It includes:

a physical allocator based on a bitmap,
a virtual memory layer using multiple segments (IdEntry),
automatic fragmentation handling,
a quarantine system preventing memory leaks,
safe Read/Write operations over virtual addresses,
and a robust best‑effort allocation strategy suitable for embedded and kernel contexts.

This allocator is intended for kernels, bare‑metal OS projects, bootloaders, embedded systems, or any environment without the standard library.

✨ Key Features
🧱 Physical Allocation (Bitmap System)

Physical memory divided into F frames.
Allocation of contiguous frame ranges when possible.
Automatic hole detection (first‑fit).
Efficient forward offset to avoid rescanning the bitmap.
Proper physical deallocation with offset recalculation.
Memory cleanup (unlock) after freeing.


🧊 Fragmentation Handling
If a contiguous block of frames cannot be allocated:

the allocator performs split allocations using a halving strategy (size / 2),
collects multiple smaller physical fragments,
and merges them virtually into a single contiguous virtual region.

This is similar to a lightweight buddy allocator.

🗂 Virtual Memory Layer
Each allocation returns a VirtualAddr:
RustVirtualAddr { id: process_id, pos: virtual_offset_in_bytes }Afficher plus de lignes

Virtual memory is always contiguous, even when physical memory is fragmented.
A virtual region may consist of one or more IdEntry segments.
Segments for the same process never overlap.
The allocator prevents forged VirtualAddr values via PID matching and segment bounds checking.


🛡 Quarantine System (Anti‑Leak Mechanism)
During deallocation:

If physical memory cannot be freed cleanly
→ the block is stored in a quarantine buffer,
→ preventing “dangling virtual entries” or leaks.

A purge() operation:

frees all quarantined blocks,
wipes the corresponding physical memory,
clears the quarantine table and counter.


📖 Read / Write API
The allocator provides safe, virtualized access to memory:
Rustread::<L>(&self, pid, addr)  -> IoStatus<L>write::<L>(&mut self, pid, addr, &[u8; L]) -> IoStatus<L>Afficher plus de lignes
Features:

automatic resolution of virtual → physical addressing,
strict boundary validation,
no cross‑page read/write (by design),
PID‑based access control,
detailed error reporting via:

ReadOk([u8; L])
WriteOk
OutOfRangeLow
OutOfRangeHigh
NoSegment


🧪 Comprehensive Test Suite (15+ tests)
Tests cover:

bitmap allocation and hole detection,
virtual allocation fragmentation,
full deallocation cycles,
quarantine behavior and purge,
Read/Write correctness,
offset handling,
PID access restrictions,
error cases (OutOfRangeHigh, NoSegment),
corruption injection (bitmap tampering).


🧬 Design Philosophy
This allocator follows a best‑effort model, ideal for no_std:

No rollback on partial allocations.
Partial allocation allowed when enabled.
Quarantine handles inconsistent physical state.
API remains deterministic, simple, panic‑free.

The goal is a clean, robust, low‑level allocator suitable for experimental OS/kernels.

🚀 Minimal Example
Rustlet mut alloc = Allocator::<4096, 64, 64>::new().unwrap();// Allocate 256 virtual bytes for process 8let (addr, status) = alloc.alloc(8, 256);// Write into virtual memorylet data = *b"Hello World!";alloc.write::<12>(8, addr.unwrap(), &data);// Read it backlet res = alloc.read::<12>(8, addr.unwrap());assert_eq!(res, IoStatus::ReadOk(data));// Deallocatealloc.desalloc(8);Afficher plus de lignes

🧩 Internal Structures

bytes: physical memory buffer (N bytes)
bitmap: allocation state of each frame
ids: virtual segments (IdEntry)
PhysFrame: (start_frame, frame_count)
VirtualAddr: process + virtual offset
quarantine: pending inconsistent physical blocks
IoStatus: results of memory IO

The entire design is deterministic, allocation‑free, and does not require the Rust standard library.