package main

import (
    "fmt"
    "log"
    //"math"
    //"go/parser"
    //"strings"
    //"strconv"
    //"image"
    //"image/color"
    //"image/draw"
    //"image/png"
    "os"
    //"os/exec"
    //"io/ioutil"
)

const (
    VIDIN = "vid_in"
    VIDOUT = "vid_out"
    IMGOUT = "img_out"
)

func checkAndCreateDir(dirName string) error {
    if _, err := os.Stat(dirName); os.IsNotExist(err) {
        err = os.MkdirAll(dirName, 0755)
        if err != nil {
            return err
        }
    }
    fmt.Printf("\ncheckAndCreateDir(): The directory %s already exists\n", dirName)
    return nil
}

func preRoutineCheck(videoInputName,framesDirName string) {
    err := checkAndCreateDir(VIDIN)
    if err != nil {
        log.Fatalf("preRoutineCheck(): ERROR :: Failed to create vid_in: %v", err)
    }
    _, err = os.Stat(VIDIN+"/"+videoInputName)
    if os.IsNotExist(err) {
        log.Fatalf("preRoutineCheck(): ERROR :: Input video file vid_in/%s does not exist", videoInputName)
    }
    err = checkAndCreateDir(VIDOUT)
    if err != nil {
        log.Fatalf("preRoutineCheck(): ERROR :: Failed to create vid_out: %v", err)
    }
    err = checkAndCreateDir(IMGOUT)
    if err != nil {
        log.Fatalf("preRoutineCheck(): ERROR :: Failed to create img_out: %v", err)
    }
    err = checkAndCreateDir(IMGOUT+"/"+framesDirName)
    if err != nil {
        log.Fatalf("preRoutineCheck(): ERROR :: Failed to create img_out/%s: %v", framesDirName, err)
    }
    fmt.Printf("\npreRoutineCheck(): All relevant IO directories and input video file %s exists\n", videoInputName)
}

/*func clamp(value,min,max int) int {
    if value < min {
        return min
    }
    if value > max {
        return max
    }
    return value
}

func distort(x,y,w,h int, amp,freq,phase float64) {
    dx := y + int(amp * math.Sin(freq * float64(x) + phase))
    dy := x + int(amp * math.Sin(freq * float64(y) + phase))
	dx = clamp(dx, 0, w-1)
	dy = clamp(dy, 0, h-1)
    return dx,dy
}*/

func videoFxRoutine(videoInName,framesDirName string) {
    preRoutineCheck(videoInName,framesDirName)
}

func main() {
    videoFxRoutine("ants.mp4", "test")
}
