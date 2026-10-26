"""Packs PNG files into a Windows .ico (PNG-compressed entries, Vista+ format)."""
import struct, sys
out, pngs = sys.argv[1], sys.argv[2:]
images = []
for path in pngs:
    data = open(path, 'rb').read()
    w, h = struct.unpack('>II', data[16:24])  # PNG IHDR width/height
    images.append((w, h, data))
header = struct.pack('<HHH', 0, 1, len(images))
offset = 6 + 16 * len(images)
entries, blobs = b'', b''
for w, h, data in images:
    entries += struct.pack('<BBBBHHII', w % 256, h % 256, 0, 0, 1, 32, len(data), offset)
    offset += len(data)
    blobs += data
open(out, 'wb').write(header + entries + blobs)
print('wrote', out, [f'{w}x{h}' for w, h, _ in images])
