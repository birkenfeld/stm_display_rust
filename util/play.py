import drawlib
import serial
import sys

s = serial.Serial(sys.argv[1], baudrate=115200)
d = drawlib.Display(s)
