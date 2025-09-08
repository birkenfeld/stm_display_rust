import sys
import serial
import drawlib
d = drawlib.Display(serial.Serial(sys.argv[1], baudrate=115200))
d.clear(15)
d.set_pos((120, 15))
d.image(0)
d.switch_graphics()
