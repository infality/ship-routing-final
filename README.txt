build instructions:

install the rust compiler and cargo

run the buildscript to create the executable binaries:
"./build.sh"

to run the application, you first have to create a graph-file from a .pbf/.sec file.
to create the graph-file (graph.bin) from a .pbf/.sec file you should run the following command (giving the path to the .sec file as an argument):
"./target/release/extract ../planet-coastlinespbf.sec"

this will create a coastlines.bin and a graph.bin in the current working-dir.
if the process is aborted before the graph.bin is created, the coastlines.bin can be used to generate the graph.bin while skipping the extraction and stitching of the coastlines:
"./target/release/extract -s coastlines.bin"

the created graph.bin can then be used to run a local webserver on port 8000:
"./target/release/route graph.bin"

point your browser to http://localhost:8000 to use the ship routing application
