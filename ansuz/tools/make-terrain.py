"""Bake a sampled voxel volume to a greedy surface mesh with a repeating cell grid.
Only exposed faces are emitted; coplanar faces merge within each axis slice.
The static GLB is a render cache, not one scene object per occupied voxel.
"""
import json, math, struct, zlib
from collections import defaultdict
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent
CELL, RADIUS, LIMIT = 2.5, 120., 51
PLACES = [(-.06,.24),(.20,.36),(-.29,.43),(.40,.65),(-.48,.16)]
CENTERS = [(math.sin(x)*math.cos(y),math.sin(y),math.cos(x)*math.cos(y)) for x,y in PLACES]

def volume():
    cells = set()
    for x in range(-LIMIT,LIMIT):
        for y in range(-LIMIT,LIMIT):
            for z in range(-LIMIT,LIMIT):
                p = [(v+.5)*CELL for v in (x,y,z)]
                r2 = sum(v*v for v in p)
                if r2 < 116**2:
                    cells.add((x,y,z)); continue
                if r2 > 125**2: continue
                r = math.sqrt(r2)
                n = [v/r for v in p]
                hill = max(4*math.exp(-sum((a-b)**2 for a,b in zip(n,c))/.007) for c in CENTERS)
                height = hill + .8*math.sin(p[0]*.12)*math.cos(p[1]*.15)
                if r <= RADIUS+height: cells.add((x,y,z))
    return cells

def mesh(cells):
    masks = defaultdict(set)
    for cell in cells:
        for axis in range(3):
            u,v = (axis+1)%3,(axis+2)%3
            for sign in (-1,1):
                neighbor=list(cell); neighbor[axis]+=sign
                if tuple(neighbor) not in cells:
                    masks[axis,sign,cell[axis]+(sign==1)].add((cell[u],cell[v]))
    exposed=sum(map(len,masks.values()))
    positions,normals,uvs,indices=[],[],[],[]
    for (axis,sign,plane),mask in sorted(masks.items()):
        u,v=(axis+1)%3,(axis+2)%3
        while mask:
            a,b=min(mask); width=1
            while (a+width,b) in mask: width+=1
            height=1
            while all((a+i,b+height) in mask for i in range(width)): height+=1
            mask.difference_update((a+i,b+j) for i in range(width) for j in range(height))
            corners=[(a,b),(a+width,b),(a+width,b+height),(a,b+height)]
            if sign<0: corners.reverse()
            start=len(positions)//3
            for cu,cv in corners:
                p=[0.,0.,0.];p[axis]=plane*CELL;p[u]=cu*CELL;p[v]=cv*CELL
                n=[0.,0.,0.];n[axis]=sign
                positions.extend(p);normals.extend(n);uvs.extend((cu,cv))
            indices.extend(start+i for i in (0,1,2,0,2,3))
    return positions,normals,uvs,indices,exposed

def png(pixel=False):
    def chunk(kind,data): return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
    rows=b''.join(b'\0'+b''.join(bytes([255 if pixel else (96 if x==0 or y==0 else 33)]*3+[255]) for x in range(32)) for y in range(32))
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',32,32,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b'')

def bake():
    (ROOT/'art/pixel.png').write_bytes(png(pixel=True))
    cells=volume();p,n,uv,i,exposed=mesh(cells)
    binary=bytearray();views=[]
    for data in (struct.pack('<%sf'%len(p),*p),struct.pack('<%sf'%len(n),*n),struct.pack('<%sf'%len(uv),*uv),struct.pack('<%sI'%len(i),*i),png()):
        views.append({'buffer':0,'byteOffset':len(binary),'byteLength':len(data)})
        binary.extend(data);binary.extend(b'\0'*(-len(binary)%4))
    accessors=[{'bufferView':k,'componentType':5125 if k==3 else 5126,'count':len(values)//size,'type':typ} for k,values,size,typ in [(0,p,3,'VEC3'),(1,n,3,'VEC3'),(2,uv,2,'VEC2'),(3,i,1,'SCALAR')]]
    accessors[0].update(min=[min(p[a::3]) for a in range(3)],max=[max(p[a::3]) for a in range(3)])
    doc={'asset':{'version':'2.0'},'scene':0,'scenes':[{'nodes':[0]}],'nodes':[{'mesh':0}], 'meshes':[{'primitives':[{'attributes':{'POSITION':0,'NORMAL':1,'TEXCOORD_0':2},'indices':3,'material':0}]}], 'materials':[{'pbrMetallicRoughness':{'baseColorTexture':{'index':0},'metallicFactor':0,'roughnessFactor':1},'emissiveTexture':{'index':0},'emissiveFactor':[.6]*3}], 'textures':[{'sampler':0,'source':0}], 'samplers':[{'magFilter':9728,'minFilter':9987,'wrapS':10497,'wrapT':10497}], 'images':[{'bufferView':4,'mimeType':'image/png'}], 'buffers':[{'byteLength':len(binary)}], 'bufferViews':views,'accessors':accessors}
    j=json.dumps(doc,separators=(',',':')).encode();j+=b' '*(-len(j)%4)
    (ROOT/'art/terrain.glb').write_bytes(struct.pack('<III',0x46546c67,2,28+len(j)+len(binary))+struct.pack('<II',len(j),0x4e4f534a)+j+struct.pack('<II',len(binary),0x004e4942)+binary)
    stats={'occupied_voxels':len(cells),'naive_cube_faces':len(cells)*6,'exposed_cell_faces':exposed,'merged_quads':len(i)//6,'triangles':len(i)//3,'cell_size':CELL}
    (ROOT/'art/terrain-stats.json').write_text(json.dumps(stats,indent=2)+'\n')
    print(json.dumps(stats))
if __name__=='__main__': bake()
