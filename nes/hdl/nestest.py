import pathlib
import sys

path = pathlib.Path(sys.argv[1])
started = False
values = []
pcs = []
num_bytes = []
val_a = []
val_x = []
val_y = []
val_p = []
val_sp = []
val_cycles = []
with open(path, "r") as file:
	for line in file:
		tokens = line.split(" ", 100)
		pcs.append(tokens[0])
		nb = 1
		b1 = int(tokens[2], 16)
		try:
			b2 = int(tokens[3], 16)
			nb = 2
		except:
			nb = 1
		try:
			if num_bytes == 2:
				b2 = int(tokens[4], 16)
				nb = 3
		except:
			nb = 2
		num_bytes.append(nb)
		check = -1
		for n in range(len(tokens)):
			if tokens[n].startswith("A:"):
				check = n
		val_a.append(tokens[check][2:])
		val_x.append(tokens[check+1][2:])
		val_y.append(tokens[check+2][2:])
		val_p.append(tokens[check+3][2:])
		val_sp.append(tokens[check+4][3:])
		check = -1
		for n in range(len(tokens)):
			if tokens[n].startswith("CYC:"):
				check = n
		val_cycles.append(int(tokens[check][4:]))
with open("nestest.txt", "w") as f:
	for i in range(len(pcs)):
		f.write(pcs[i])
		f.write(" " + '{:x}'.format(num_bytes[i]))
		f.write(" " + val_a[i])
		f.write(" " + val_x[i])
		f.write(" " + val_y[i])
		f.write(" " + val_p[i])
		f.write(" " + val_sp[i])
		f.write(" " + '{:04X}'.format(val_cycles[i]))
		f.write(" 42\n")