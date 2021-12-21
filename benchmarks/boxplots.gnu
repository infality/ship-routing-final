set style fill solid 0.5 border -1
set style boxplot outliers pointtype 7
set style data boxplot
set boxwidth  0.5
set pointsize 0.5
FILES = system("ls -1 *.txt")
plot for [i=1:words(FILES)] word(FILES,i) u (i):($1) title word(FILES,i)
