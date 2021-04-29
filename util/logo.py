import sys
import serial
import drawlib
d = drawlib.Display(serial.Serial(sys.argv[1]))
d.clear(15)
d.set_pos((120, 15))
d.icon(0)
d.switch_graphics()
