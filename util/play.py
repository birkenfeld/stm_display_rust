import drawlib
import serial

s = serial.Serial('/dev/ttyUSB0', baudrate=115200)
d = drawlib.Display(s)
