#!/usr/bin/env python3

import sys
import serial

from drawlib import Display

d = Display(serial.Serial(sys.argv[1], baudrate=115200))

d.switch_console()
