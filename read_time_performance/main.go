package main

import (
	"flag"
	"fmt"
	"os"
	"time"
)

func main() {
	var block_size uint
	flag.UintVar(&block_size, "bsize", 100, "Block size to read")

	var read_repetition uint
	flag.UintVar(&read_repetition, "repeat", 1, "Read repetition")

	var file_path string
	flag.StringVar(&file_path, "fpath", ".", "File path")

	var log_path string
	flag.StringVar(&log_path, "lpath", "/tmp/read_log", "File path")

	flag.Parse()

	log_file, _ := os.Create(log_path)
	repetition_avg := uint(0)
	min_read := ^uint(0)
	max_read := uint(0)
	for loop_index := read_repetition; loop_index > 0; loop_index-- {

		total_read_time, avg_op_time := handle_read(file_path, block_size)
		repetition_avg += total_read_time
		sout := fmt.Sprintf("Read time (all blocks): %.3f us  (%.2f ms)\r\nAvrg time (for the blocks): %.3f us\r\n\r\n", float32(total_read_time), float32(total_read_time)/1000.0, avg_op_time)
		fmt.Print(sout)
		if total_read_time < min_read {
			min_read = total_read_time
		}

		if total_read_time > max_read {
			max_read = total_read_time
		}
		log_file.WriteString(sout)
	}
	repetition_avg /= read_repetition
	sout := fmt.Sprintf("======[%d times the file read | %d Bytes block size]=========\r\nAverage read time : %d us (%.2f ms)\r\nMin read time: %d us (%.2f ms)\r\nMax read time: %d us (%.2f ms)\r\n\r\n", read_repetition, block_size, repetition_avg, float32(repetition_avg)/1000.0, min_read, float32(min_read)/1000.0, max_read, float32(max_read)/1000.0)
	fmt.Print(sout)
	log_file.WriteString(sout)
}

func handle_read(file_path string, block_size uint) (total_read_time uint, avg_op_time float32) {
	f, err := os.Open(file_path)

	if err != nil {
		panic(fmt.Sprintln("Err: ", err))
	}

	buf := make([]byte, block_size)
	read_count := uint(0)
	var before_read int64
	var after_read int64
	read_bytes := int(^uint(0) >> 1) //max int
	for read_bytes > 0 {
		before_read = time.Now().UnixMicro()
		read_bytes, _ = f.Read(buf)
		after_read = time.Now().UnixMicro()
		block_read_time := uint(after_read - before_read)
		total_read_time += block_read_time
		read_count++
	}
	avg_op_time = float32(total_read_time) / float32(read_count)

	return
}
