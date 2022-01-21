import matplotlib.pyplot as plt
import numpy as np
import os
from pathlib import Path

benchmark_dir = os.path.dirname(os.path.realpath(__file__))
os.chdir(benchmark_dir)

files = filter(os.path.isfile, os.listdir(benchmark_dir))
files = filter(lambda x: x.endswith(".txt"), files)
files = [os.path.join(benchmark_dir, f) for f in files]
files.sort(key=lambda x: os.path.getmtime(x))

names = []
data = []
for path in files:
    with open(path) as file:
        names.append(Path(path).stem)
        data.append(list(map(lambda x: float(x.rstrip()), file.readlines())))

plt.violinplot(data, showextrema=False)
plt.boxplot(data, widths=0.15, patch_artist=True, boxprops=dict(color="#3b7cab"))
plt.xticks(range(1, len(names) + 1), names)
plt.ylabel("ms per query")
plt.grid(axis="y")
plt.show()
