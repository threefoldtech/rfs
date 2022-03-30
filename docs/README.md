# Prerequests


If the term User space is strange check this -> [**Kernel space** and **User space**](https://en.wikipedia.org/wiki/User_space_and_kernel_space)


**FS** (File System): Is a way of managing How to store and retrive data, such as ext4 and NTFS.

**How files stored and retrieved**: 

- in linux, every file has an associated what called <u>Inode</u>
- the Inode is a data structure which contains the following
  
    - the file's metadata
    - 12 direct pointers to 12 data blocks from the file
    - indirect pointer to a 12 direct pointers which points to the next 12 data blocks
    - two doubly indirect pointers, each one points to an indirect pointer and that indirect pointer points to the next 12 data blocs 
    - three tribly indirect pointers .. you know the idea :)


<u>Inode structure</u>

    | meta data |

    |    12     |   ----> data blocks (0)
    |  direct   |   ---->     ...
    | pointers  |   ----> data blocks (11)

    |indirectPtr|   ---->   |    12     |   ----> data blocks (12)
                            |  direct   |   ---->     ...
                            | pointers  |   ----> data blocks (23)

    |two doubly |   ---->   |indirectPtr|   ---->   |    12     |   ----> data blocks (24)
    |indirectPtr|   -----                           |  direct   |   ---->     ...
                        |                           | pointers  |   ----> data blocks (23)
                        |
                        |
                        --> |indirectPtr|   ---->   |    12     |   ----> data blocks (24)
                                                    |  direct   |   ---->     ...
                                                    | pointers  |   ----> data blocks (23)

    |  three    |   ---->   |two doubly indirectPtr| ....
    |  tribly   |   ---->   |two doubly indirectPtr| ....
    |indirectPtr|   ---->   |two doubly indirectPtr| ....

**VFS** (Virutal File System): is an abstraction layer on the actual mounted filesystems. The user space sees only the virtual file system and the VFS manages the underlying FileSystems. <u>so that</u> any file system must register at the VFS.

**FUSE** (File system in USEr space): Is a user space filesystem framework (consists of two parts) used to build your own file-system (a user-defined file-system).


**FUSE main components**:

 - FUSE file system daemon (with FUSE library i.e. libfuse)
 - FUSE driver (with a request queue)

These two componenet are communicates through `/dev/fuse` device, it works as IPC (Inter-Process Communication).

**How FUSE works**:

![image](https://user-images.githubusercontent.com/18401282/160552509-d40ab27a-a002-4fae-a6b6-fb983f97babf.png)



- suppose we take the `/tmp/rmnt` directory as a mount point for our own file system (i.e rfs)
- The fuse driver registeres `/tmp/rmnt` as a mount point in the VFS
- when you inside `/tmp/rmnt` and you run and application such as `ls` (which means you need to read the content of the current directory)
- This will need to a call to the VFS
- The VFS knows that the `/tmp/rmnt` is related to the fuse driver, so that the VFS routes the operation to it.
- the driver allocates a FUSE request structure and puts it in the FUSE queue in a wait state
- the FUSE daemon then picks the request from the kernel queue by reading from `/dev/fuse`.
- the userspace filesystem (i.e. `rfs`) will read the request and process it.

---
# rfs (Rust File-System)
rfs (Rust File-Sytems): is a FUSE file system which used in Zero-OS.

**The main idea** of `rfs` is reading a remote file as needed chunk by chunk.

<u>Explanation</u>:

- your files is stored on a remote server which is [hub.grid.tf](hub.grid.tf)
- and you want locally read a file or a video locally
- rfs will only download the file or the video in chunks (not the whole file or video)
- every chunk is actually a file with a hash that represent the id of that chunk
- the downloaded chunks is saved in a cache (it is actually a directory on your local machine i.e. `/tmp/cache`)

<u> But how rfs knows about the structure of the remote server's filesystem </u>

- the server [hub.grid.tf](hub.grid.tf) recieves a `.tar.gz` file.
- It containes the whole file system structure with its contents.
- after uploading the `.tar.gz` file, the server builds a `.flist` file (accronym File LIST)
- This `.flist` file is a sqlite database with the file system structure without the actual contents (only the names of the directories and the files).


**The interaction between `rfs` and the `FUSE daemon`**

- when the fuse daemon reading the request from the kernel queue by reading from `/dev/fuse`.
- `rfs` reads the request and checkes which operation want to be served (the operation must be implemented to be served)
- after handling the request the `rfs` replies with the result

<u>Handling the read operation</u>


- The request wants for example to read 400B from position x (where x < 400 and chunk size is 100B as an example)
- Calculating the cursor offset and the chunk index (offset = x, chunk_index = x/chunk_size)
- Reading the chunk (chunk_index) from the cache, But if the chunk is not in the cache, then it needs to be downloaded and cached
- repeat the previous step until requested size fullfilled and response to the FUSE request


<u>Use Case</u>
(suppose that every block is 100B)

    |-------0--------|--------1---------|---------2-------|---------3-------|--------4--------|

                              ^--------offset = 150 && size = 250 - ---^

- chunk_index = offset/chunk_size = 150/100 = 1
- here the read operation will start from block (1) untile block(3) to fulfill the requested size
- if such a block was downloaded it will be found in the cache and read
- if the block is not in the cache, it will be downloaded from the server


---
# rfs Usage
    
    rfs --help

    USAGE:
        rfs [FLAGS] [OPTIONS] <TARGET> --meta <META>
    
    FLAGS:
        -d, --daemon     daemonize process
            --debug      enable debug logging
        -h, --help       Prints help information
        -V, --version    Prints version information
    
    OPTIONS:
            --cache <cache>        cache directory [default: /tmp/cache]
            --storage-url <hub>    storage url to retrieve files from [default: redis://hub.grid.tf:9900]
            --log <log>            log file only in daemon mode
            --meta <META>          metadata file, can be a .flist file, a .sqlite3 file or a directory with a
                                   `flistdb.sqlite3` inside
    
    ARGS:
        <TARGET>



### Use case

    rfs --meta <filename>.flist /tmp/rmnt
