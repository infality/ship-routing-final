set style fill solid 0.5 border -1
set style boxplot outliers pointtype 7
set style data boxplot
set boxwidth  0.5
set pointsize 0.5
set ylabel "ms per query"
set grid
unset xtics
FILES = system("ls -1tr *.txt")
plot for [i=1:words(FILES)] word(FILES,i) u (i):($1) title substr(word(FILES,i), 0, strstrt(word(FILES,i), ".txt")-1)
