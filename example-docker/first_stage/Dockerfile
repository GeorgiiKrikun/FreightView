FROM ubuntu:22.04

RUN mkdir -p /home/georgii
RUN echo "Hello, Georgii!" > /home/georgii/hello.txt
RUN echo "Hello, Georgii2!" > /home/georgii/hello2.txt
RUN rm /home/georgii/hello.txt
RUN mkdir -p /home/georgii/dir1
RUN echo "Hello, Georgii from dir1!" > /home/georgii/dir1/hello.txt
RUN rm -rf /home/georgii/dir1

# # To make file size larger
# RUN dd if=/dev/zero of=/home/georgii/large_file_1.dat bs=1G count=2 status=progress
# RUN dd if=/dev/zero of=/home/georgii/large_file_2.dat bs=1G count=2 status=progress
# RUN dd if=/dev/zero of=/home/georgii/large_file_3.dat bs=1G count=2 status=progress
# RUN dd if=/dev/zero of=/home/georgii/large_file_4.dat bs=1G count=2 status=progress
# RUN dd if=/dev/zero of=/home/georgii/large_file_5.dat bs=1G count=2 status=progress
#


