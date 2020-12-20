import json
import bpy

def bezier_points(spline):
    bps = [
        o.data.splines[0].bezier_points
            for o in bpy.data.objects
            if o.name == spline
    ][0]
    return [
        {
            'left': bp.handle_left[:2],
            'pos': bp.co[:2],
            'right': bp.handle_right[:2]
        }
            for bp in bps
    ]

with open('./track.json', 'w') as fp:
    paths = [
        bezier_points(o.name)
            for o in bpy.data.objects
            if o.name.startswith("track")
    ]
    
    json.dump(paths, fp)