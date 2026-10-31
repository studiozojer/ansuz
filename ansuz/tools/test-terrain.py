import importlib.util
from pathlib import Path
import unittest
spec=importlib.util.spec_from_file_location('terrain',Path(__file__).with_name('make-terrain.py'))
terrain=importlib.util.module_from_spec(spec);spec.loader.exec_module(terrain)
class Meshing(unittest.TestCase):
    def test_adjacent_cells_merge_to_six_faces(self):
        p,n,uv,i,exposed=terrain.mesh({(0,0,0),(1,0,0)})
        self.assertEqual(exposed,10)
        self.assertEqual(len(i)//6,6)
        area=0
        for q in range(0,len(p),12):
            a,b,c=[p[q+j:q+j+3] for j in (0,3,6)]
            u=[b[k]-a[k] for k in range(3)];v=[c[k]-b[k] for k in range(3)]
            cross=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
            self.assertGreater(sum(x*y for x,y in zip(cross,n[q:q+3])),0)
            area+=sum(x*x for x in cross)**.5
        self.assertAlmostEqual(area,10*terrain.CELL**2)
    def test_interior_cavity_is_preserved(self):
        cells={(x,y,z) for x in range(3) for y in range(3) for z in range(3)}-{(1,1,1)}
        *_,indices,exposed=terrain.mesh(cells)
        self.assertEqual(exposed,60)
        self.assertEqual(len(indices)//6,12)
if __name__=='__main__':unittest.main()
