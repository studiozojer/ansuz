"""A small, closed clay slab: rounded perimeter, bevels and a softly uneven face.
No writing is baked in; this is a presentation of whichever surface is selected.
"""
import json
import math
import struct
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEGMENTS = 64
positions, normals, colors, indices = [], [], [], []


def point(radius, angle, y):
    c, s = math.cos(angle), math.sin(angle)
    # A superellipse keeps the slab square-ish without sharp manufactured corners.
    wobble = 1 + .009 * math.sin(7 * angle) + .006 * math.cos(11 * angle)
    return (2 * radius * math.copysign(abs(c) ** .28, c) * wobble,
            y, 2.5 * radius * math.copysign(abs(s) ** .28, s) * wobble)


def triangle(a, b, c, shade):
    b, c = c, b
    ab, ac = [b[i] - a[i] for i in range(3)], [c[i] - a[i] for i in range(3)]
    n = [ab[1]*ac[2]-ab[2]*ac[1], ab[2]*ac[0]-ab[0]*ac[2], ab[0]*ac[1]-ab[1]*ac[0]]
    length = math.sqrt(sum(v*v for v in n))
    if length < 1e-10:
        return
    start = len(positions) // 3
    for p in (a, b, c):
        positions.extend(p)
        normals.extend(v / length for v in n)
        grain = .025 * math.sin(p[0]*79 + p[2]*37) * math.cos(p[0]*43 - p[2]*83)
        colors.extend((shade+grain, (shade+grain)*.97, (shade+grain)*.91, 1))
    indices.extend((start, start+1, start+2))


def ring(radius, y, uneven=False):
    result = []
    for i in range(SEGMENTS):
        x, _, z = point(radius, i*math.tau/SEGMENTS, y)
        ripple = .012 * math.sin(x*3.2) * math.cos(z*2.7) if uneven else 0
        result.append((x, y + ripple, z))
    return result


# Traversal from the back centre to the front centre gives outward-facing triangles.
rings = [ring(.001, -.25)]
for k in range(1, 9):
    r = .90*k/8
    rings.append(ring(r, -.25 - .025*(1-r*r), True))
rings.extend([ring(1, -.12, True), ring(1, .055, True), ring(.91, .20, True)])
for k in range(7, 0, -1):
    r = .91*k/8
    rings.append(ring(r, .20 + .13*(1-(r/.91)**2), True))
rings.append(ring(.001, .33))
for k, (a, b) in enumerate(zip(rings, rings[1:])):
    shade = .66 if k < 8 else (.48 if k < 10 else .78)
    for i in range(SEGMENTS):
        j = (i+1) % SEGMENTS
        triangle(a[i], a[j], b[j], shade)
        triangle(a[i], b[j], b[i], shade)
# Close the tiny centre holes too.
for i in range(1, SEGMENTS-1):
    triangle(rings[0][0], rings[0][i+1], rings[0][i], .66)
    triangle(rings[-1][0], rings[-1][i], rings[-1][i+1], .78)

# Shared shading normals keep the face from showing the radial tessellation.
shared = defaultdict(lambda: [0., 0., 0.])
for i in range(0, len(positions), 3):
    key = tuple(positions[i:i+3])
    for axis in range(3):
        shared[key][axis] += normals[i+axis]
for i in range(0, len(positions), 3):
    n = shared[tuple(positions[i:i+3])]
    length = math.sqrt(sum(v*v for v in n))
    normals[i:i+3] = [v/length for v in n]

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
print(f'tablet.glb: {len(indices)//3} triangles, {len(result)} bytes')
