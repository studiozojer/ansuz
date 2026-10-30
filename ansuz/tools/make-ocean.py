"""Generate Ansuz's smooth, untextured ocean mesh. No external dependencies."""
import json, math, struct
from pathlib import Path
positions, indices = [], []
rows, cols = 96, 192
for y in range(rows + 1):
    a = math.pi * y / rows
    for x in range(cols + 1):
        b = math.tau * x / cols
        positions.extend([120 * math.sin(a) * math.cos(b), 120 * math.cos(a), 120 * math.sin(a) * math.sin(b)])
for y in range(rows):
    for x in range(cols):
        a = y * (cols + 1) + x
        b = a + cols + 1
        indices.extend([a, a + 1, b, a + 1, b + 1, b])
p = struct.pack('<%sf' % len(positions), *positions)
n = struct.pack('<%sf' % len(positions), *(v / 120 for v in positions))
i = struct.pack('<%sI' % len(indices), *indices)
binary = p + n + i
rgb = [((v / 255 + .055) / 1.055) ** 2.4 for v in [35, 73, 75]]
doc = {'asset': {'version': '2.0'}, 'scene': 0, 'scenes': [{'nodes': [0]}], 'nodes': [{'mesh': 0}], 'meshes': [{'primitives': [{'attributes': {'POSITION': 0, 'NORMAL': 1}, 'indices': 2, 'material': 0}]}], 'materials': [{'pbrMetallicRoughness': {'baseColorFactor': rgb + [1], 'metallicFactor': 0, 'roughnessFactor': .9}}], 'buffers': [{'byteLength': len(binary)}], 'bufferViews': [{'buffer': 0, 'byteOffset': offset, 'byteLength': length} for offset, length in [(0, len(p)), (len(p), len(n)), (len(p)+len(n), len(i))]], 'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': len(positions)//3, 'type': 'VEC3', 'min': [-120]*3, 'max': [120]*3}, {'bufferView': 1, 'componentType': 5126, 'count': len(positions)//3, 'type': 'VEC3'}, {'bufferView': 2, 'componentType': 5125, 'count': len(indices), 'type': 'SCALAR'}]}
j = json.dumps(doc, separators=(',', ':')).encode()
j += b' ' * (-len(j) % 4)
output = struct.pack('<III', 0x46546c67, 2, 28+len(j)+len(binary)) + struct.pack('<II', len(j), 0x4e4f534a) + j + struct.pack('<II', len(binary), 0x004e4942) + binary
(Path(__file__).resolve().parent.parent / 'art/ocean.glb').write_bytes(output)
