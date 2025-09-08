import sys
import math
import time

import serial
import PIL.Image

import drawlib

d = drawlib.Display(serial.Serial(sys.argv[1], baudrate=115200))
img = PIL.Image.open(sys.argv[2])
try:
    scale = int(sys.argv[3])
except Exception:
    scale = 1

palette = sum(map(list, drawlib.Colors.LUT), [])

p_img = PIL.Image.new('P', img.size)
p_img.putpalette(palette)

w = min(img.size[0] * scale, 480)
h = min(img.size[1] * scale, 128)
npx = math.ceil(w / scale)
npy = math.ceil(h / scale)
scxy = (scale, scale)

frames = []
for frame in range(getattr(img, 'n_frames', 1)):
    img.seek(frame)
    conv = img.convert('RGB').quantize(palette=p_img)
    frames.append([])
    for y in range(npy):
        frames[-1].append(bytes(
            conv.getpixel((x, y)) for x in range(min(npx, 240))
        ))
        frames[-1].append(bytes(
            conv.getpixel((x, y)) for x in range(240, npx)
        ))

d.clear(0)
d.switch_graphics()

while True:
    for frame in frames:
        for y in range(npy):
            d.pixels((0, y*scale), (min(npx, 240), 1), scxy, frame[2*y])
            if npx > 240:
                d.pixels((240*scale, y*scale), (npx - 240, 1), scxy, frame[2*y+1])
        time.sleep(0.1)
