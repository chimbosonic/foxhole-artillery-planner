#!/usr/bin/env python3
"""Extract hex map PNGs from a Foxhole improved-map-mod .pak file (UE4 pak v3)."""

import struct
from pathlib import Path

import texture2ddecoder
from PIL import Image

PAK_PATH = Path(__file__).parent / "War-WindowsNoEditor_IMM_NoRDZ-Multi.pak"
OUT_DIR = Path(__file__).parent / "hex_pngs"


# ---------------------------------------------------------------------------
# UE4 Pak v3 reader
# ---------------------------------------------------------------------------


def read_fstring_buf(data, pos):
    """Read a UE4 FString from a buffer at given position. Returns (string, new_pos)."""
    length = struct.unpack_from("<i", data, pos)[0]
    pos += 4
    if length == 0:
        return "", pos
    if length > 0:
        s = data[pos : pos + length].rstrip(b"\x00").decode("utf-8", errors="replace")
        return s, pos + length
    else:
        s = (
            data[pos : pos + (-length * 2)]
            .decode("utf-16-le", errors="replace")
            .rstrip("\x00")
        )
        return s, pos + (-length * 2)


def read_pak_index(pak_data, version, index_offset, index_size):
    """Parse the pak index. Returns list of (filename, offset, comp_size, uncomp_size)."""
    idx = pak_data[index_offset : index_offset + index_size]
    p = 0

    mount, p = read_fstring_buf(idx, p)
    num_entries = struct.unpack_from("<i", idx, p)[0]
    p += 4

    entries = []
    for _ in range(num_entries):
        fname, p = read_fstring_buf(idx, p)
        offset, comp_size, uncomp_size = struct.unpack_from("<QQQ", idx, p)
        p += 24
        comp_method = struct.unpack_from("<I", idx, p)[0]
        p += 4
        p += 20  # sha1
        num_blocks = struct.unpack_from("<I", idx, p)[0]
        p += 4
        p += num_blocks * 16  # compression blocks
        p += 1  # flags byte

        entries.append((fname, offset, comp_size, uncomp_size, comp_method))

    return mount, entries


def read_pak_footer(pak_data):
    """Read pak footer from end of data. Returns (version, index_offset, index_size)."""
    tail = pak_data[-221:]
    magic = b"\xe1\x12\x6f\x5a"
    pos = tail.rfind(magic)
    if pos < 0:
        raise ValueError("Could not find pak magic in footer")
    footer = tail[pos:]
    _, version = struct.unpack_from("<II", footer, 0)
    index_offset, index_size = struct.unpack_from("<QQ", footer, 8)
    return version, index_offset, index_size


def get_entry_data(pak_data, offset, version):
    """Read entry data from pak at given offset, skipping the per-entry header."""
    p = offset
    # Per-entry header: offset(8) + comp(8) + uncomp(8) + method(4) + sha1(20) + num_blocks(4) + blocks + flags(1)
    p += 8 + 8 + 8 + 4 + 20  # skip fixed fields
    num_blocks = struct.unpack_from("<I", pak_data, p)[0]
    p += 4
    p += num_blocks * 16
    p += 1  # flags

    uncomp_size = struct.unpack_from("<Q", pak_data, offset + 16)[0]
    return pak_data[p : p + uncomp_size]


# ---------------------------------------------------------------------------
# Texture extraction from uasset data
# ---------------------------------------------------------------------------

BC_DECODERS = {
    "PF_DXT1": texture2ddecoder.decode_bc1,
    "PF_DXT5": texture2ddecoder.decode_bc3,
    "PF_BC4": texture2ddecoder.decode_bc4,
    "PF_BC5": texture2ddecoder.decode_bc5,
    "PF_BC6H": texture2ddecoder.decode_bc6,
    "PF_BC7": texture2ddecoder.decode_bc7,
}


def extract_texture(uasset_data):
    """
    Extract the largest mip texture from a cooked UE4 Texture2D uasset.
    Returns (PIL.Image, pixel_format_str) or (None, None).

    Strategy:
    1. Find "PF_" pixel format string in serialized data (after TotalHeaderSize)
    2. Read SizeX, SizeY from just before the FString
    3. Find bulk data flags (0x48 or similar) after the PF string
    4. Read element count, size, and offset to locate texture bytes
    5. Decode using texture2ddecoder
    """
    # Get TotalHeaderSize from package header
    # Magic(4) + LegacyFileVersion(4) = offset 8
    # Then varying fields, but TotalHeaderSize is always at a fixed offset for v-7 headers
    magic = struct.unpack_from("<I", uasset_data, 0)[0]
    if magic != 0x9E2A83C1:
        return None, None

    # Find TotalHeaderSize: scan early ints for a plausible value (100-10000)
    # For -7 format, it's at offset 24
    total_hdr = struct.unpack_from("<i", uasset_data, 24)[0]
    if not (50 < total_hdr < 50000):
        # Try other common offsets
        for try_off in [20, 28, 32]:
            total_hdr = struct.unpack_from("<i", uasset_data, try_off)[0]
            if 50 < total_hdr < 50000:
                break
        else:
            total_hdr = 100  # fallback: search from early in the file

    # Find "PF_" in the serialized data portion
    pf_idx = uasset_data.find(b"PF_", total_hdr)
    if pf_idx == -1:
        # Try searching from start (in case TotalHeaderSize was wrong)
        pf_idx = uasset_data.find(b"PF_", 100)
    if pf_idx == -1:
        return None, None

    # Read the FString length prefix
    fstr_len = struct.unpack_from("<i", uasset_data, pf_idx - 4)[0]
    if fstr_len <= 0 or fstr_len > 30:
        return None, None

    pf_name = uasset_data[pf_idx : pf_idx + fstr_len - 1].decode(
        "utf-8", errors="replace"
    )
    if pf_name not in BC_DECODERS and pf_name != "PF_B8G8R8A8":
        print(f"  Unsupported pixel format: {pf_name}")
        return None, None

    # Read dimensions from before the FString
    # Layout: SizeX(4) + SizeY(4) + PackedData(4) + FStringLen(4) + FStringData(N)
    dim_base = pf_idx - 4 - 12  # go back past FStringLen, PackedData, SizeY to SizeX
    sx = struct.unpack_from("<i", uasset_data, dim_base)[0]
    sy = struct.unpack_from("<i", uasset_data, dim_base + 4)[0]

    if sx <= 0 or sy <= 0 or sx > 16384 or sy > 16384:
        return None, None

    # Find bulk data flags after the PF string
    # Scan for byte 0x48 (ForceInline | ForceSingleElement) or other valid bulk flags
    after_pf = pf_idx + fstr_len
    bulk_offset = None

    for i in range(after_pf, min(after_pf + 40, len(uasset_data) - 20)):
        candidate = struct.unpack_from("<I", uasset_data, i)[0]
        if candidate in (0x48, 0x40, 0x41, 0x49, 0x09, 0x01, 0x08):
            # Verify: next int32 should be element count matching texture size
            ec = struct.unpack_from("<i", uasset_data, i + 4)[0]
            if ec == sx * sy:
                bulk_offset = i
                break

    if bulk_offset is None:
        # Try int64 element count (newer UE4 versions)
        for i in range(after_pf, min(after_pf + 40, len(uasset_data) - 28)):
            candidate = struct.unpack_from("<I", uasset_data, i)[0]
            if candidate in (0x48, 0x40, 0x41, 0x49, 0x09, 0x01, 0x08):
                ec = struct.unpack_from("<q", uasset_data, i + 4)[0]
                if ec == sx * sy:
                    bulk_offset = i
                    break

    if bulk_offset is None:
        return None, None

    # Parse bulk data header
    bulk_flags = struct.unpack_from("<I", uasset_data, bulk_offset)[0]
    element_count = struct.unpack_from("<i", uasset_data, bulk_offset + 4)[0]
    size_on_disk = struct.unpack_from("<i", uasset_data, bulk_offset + 8)[0]
    file_offset = struct.unpack_from("<q", uasset_data, bulk_offset + 12)[0]

    # Texture data is inline (flags 0x48 = ForceInline)
    # The data starts at the file_offset within the uasset
    tex_start = file_offset
    if tex_start < 0 or tex_start + size_on_disk > len(uasset_data):
        # Try: data is right after the bulk header
        tex_start = bulk_offset + 20
        if tex_start + size_on_disk > len(uasset_data):
            return None, None

    tex_data = uasset_data[tex_start : tex_start + size_on_disk]

    # Decode
    if pf_name == "PF_B8G8R8A8":
        img = Image.frombytes("RGBA", (sx, sy), tex_data, "raw", "BGRA")
    else:
        decoder = BC_DECODERS[pf_name]
        decoded = decoder(tex_data, sx, sy)
        img = Image.frombytes("RGBA", (sx, sy), decoded, "raw", "BGRA")

    return img, pf_name


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    OUT_DIR.mkdir(exist_ok=True)

    print(f"Reading {PAK_PATH.name} ({PAK_PATH.stat().st_size / 1e6:.1f} MB)...")
    pak_data = PAK_PATH.read_bytes()

    version, index_offset, index_size = read_pak_footer(pak_data)
    print(f"Pak version: {version}, {index_size} byte index")

    mount, entries = read_pak_index(pak_data, version, index_offset, index_size)
    print(f"Mount: {mount}, {len(entries)} entries")

    # Filter for hex map textures
    hex_entries = [(f, o, cs, us, cm) for f, o, cs, us, cm in entries if "HexMaps" in f]
    other_entries = [
        (f, o, cs, us, cm) for f, o, cs, us, cm in entries if "HexMaps" not in f
    ]

    print(f"\nHex map entries: {len(hex_entries)}")
    print(f"Other entries: {len(other_entries)}")
    for f, *_ in other_entries:
        print(f"  (other) {f}")

    extracted = 0
    failed = []

    for fname, offset, comp_size, uncomp_size, comp_method in sorted(entries):
        short = fname.rsplit("/", 1)[-1].replace(".uasset", "")
        print(f"\n[{short}]", end=" ")

        if comp_method != 0:
            print(f"compressed (method={comp_method}), skipping")
            failed.append(short)
            continue

        uasset = get_entry_data(pak_data, offset, version)
        img, pf = extract_texture(uasset)

        if img is None:
            print("failed to extract")
            failed.append(short)
            continue

        print(f"{img.width}x{img.height} {pf}", end=" ")
        out_path = OUT_DIR / f"{short}.png"
        img.save(out_path)
        print(f"-> {out_path.name}")
        extracted += 1

    print(f"\n{'=' * 60}")
    print(f"Extracted {extracted}/{len(entries)} textures to {OUT_DIR}/")
    if failed:
        print(f"Failed: {', '.join(failed)}")


if __name__ == "__main__":
    main()
