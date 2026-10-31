"""Bake a filled voxel clay tablet; only exposed cube faces become triangles.
The grid is the shape itself, not a pixel texture on a smoothed slab.
"""
import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CELL = .25
cells = set()
for x in range(-8, 8):
    for z in range(-10, 10):
        u, v = abs((x+.5)/8), abs((z+.5)/10)
        if u**6 + v**6 > 1:
            continue
        # A one-voxel bevel and clipped corners keep the clay silhouette stepped.
        top = 1 if u < .76 and v < .8 else 0
        for y in range(-2, top+1):
            cells.add((x, y, z))

positions, normals, colors, indices = [], [], [], []
for cell in sorted(cells):
    for axis in range(3):
        u, v = (axis+1)%3, (axis+2)%3
        for sign in (-1, 1):
            neighbour = list(cell)
            neighbour[axis] += sign
            if tuple(neighbour) in cells:
                continue
            corners = [(0,0), (1,0), (1,1), (0,1)]
            if sign < 0:
                corners.reverse()
            start = len(positions)//3
            # Quantized clay tones make individual voxels legible without gaps.
            grain = ((cell[0]*73 ^ cell[1]*31 ^ cell[2]*137) & 7) / 7
            shade = (.75 if axis == 1 and sign == 1 else .60) + .10*grain
            for a, b in corners:
                p = [float(n) for n in cell]
                p[axis] += sign == 1
                p[u] += a
                p[v] += b
                n = [0., 0., 0.]
                n[axis] = sign
                positions.extend(c*CELL for c in p)
                normals.extend(n)
                colors.extend((shade, shade*.97, shade*.91, 1))
            indices.extend(start+i for i in (0,1,2,0,2,3))

binary, views = bytearray(), []
for values, fmt in [(positions,'f'), (normals,'f'), (colors,'f'), (indices,'I')]:
    data = struct.pack('<'+str(len(values))+fmt, *values)
    views.append({'buffer':0, 'byteOffset':len(binary), 'byteLength':len(data)})
    binary.extend(data)
accessors = [{'bufferView':i, 'componentType':5125 if i==3 else 5126,
              'count':len(values)//size, 'type':kind}
             for i,values,size,kind in [(0,positions,3,'VEC3'),(1,normals,3,'VEC3'),(2,colors,4,'VEC4'),(3,indices,1,'SCALAR')]]
accessors[0].update(min=[min(positions[i::3]) for i in range(3)], max=[max(positions[i::3]) for i in range(3)])
doc = {'asset':{'version':'2.0'}, 'scene':0, 'scenes':[{'nodes':[0]}], 'nodes':[{'mesh':0}],
       'meshes':[{'primitives':[{'attributes':{'POSITION':0,'NORMAL':1,'COLOR_0':2},'indices':3,'material':0}]}],
       'materials':[{'name':'matte-clay','pbrMetallicRoughness':{'baseColorFactor':[.62,.60,.56,1],'metallicFactor':0,'roughnessFactor':1}}],
       'buffers':[{'byteLength':len(binary)}], 'bufferViews':views, 'accessors':accessors}
encoded = json.dumps(doc,separators=(',',':')).encode()
encoded += b' ' * (-len(encoded)%4)
result = struct.pack('<III',0x46546c67,2,28+len(encoded)+len(binary)) + struct.pack('<II',len(encoded),0x4e4f534a) + encoded + struct.pack('<II',len(binary),0x004e4942) + binary
(ROOT/'art/tablet.glb').write_bytes(result)
print(f'tablet.glb: {len(cells)} occupied voxels, {len(indices)//6} exposed faces, {len(indices)//3} triangles, {len(result)} bytes')
