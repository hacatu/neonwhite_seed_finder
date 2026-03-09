MSEED = 161803398
I32MAX = (1 << 31) - 1

class DotnetRandom:
	def __init__(self, seed=None):
		self.state = [0]*56
		self.i = 0
		if seed is not None:
			self.set_seed(seed)
	
	def set_seed(self, seed):
		seed = MSEED - seed
		self.state[55] = seed
		num3 = 1
		index = 0
		for _ in range(54):
			index = (index + 21)%55
			self.state[index] = num3
			num3, seed = seed-num3, num3
			num3 %= I32MAX
		for _ in range(4):
			for k in range(55):
				acc = self.state[k+1] - self.state[(k+31)%55 + 1]
				self.state[k+1] = acc%I32MAX
	
	def next(self):
		i1 = (self.i + 21)%55 + 1
		self.i = self.i%55 + 1
		num3 = self.state[self.i] - self.state[i1]
		num3 %= I32MAX
		self.state[self.i] = num3
		return num3

