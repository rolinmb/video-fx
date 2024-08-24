package main

import (
    "fmt"
    "log"
    "math"
    "go/parser"
    "strings"
    "image"
    "image/color"
    "image/png"
    "image/jpeg"
    "os"
    "os/exec"
    "io/ioutil"
    "sync"
)

const (
    VIDIN = "vid_in"
    VIDOUT = "vid_out"
    IMGOUT = "img_out"
    PNG = "png"
    JPEG = "jpg"
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
    _, err = os.Stat(IMGOUT+"/"+framesDirName)
    if os.IsNotExist(err) {
        err = os.MkdirAll(IMGOUT+"/"+framesDirName, 0755)
        if err != nil {
            log.Fatalf("preRoutineCheck: ERROR :: Failed to create img_out/%s: %v", framesDirName)
        }
    } else {
        fmt.Printf("\ncheckAndCreateDir(): The directory img_out/%s already exists; cleaning up...\n", framesDirName)
        os.RemoveAll(IMGOUT+"/"+framesDirName)
        os.MkdirAll(IMGOUT+"/"+framesDirName, 0755)
        fmt.Printf("\npreRoutineCheck(): The previous contents of directory img_out/%s were successfully cleared\n", framesDirName)
    }
    fmt.Printf("\npreRoutineCheck(): All relevant IO directories and input video file %s exist\n", videoInputName)
}

func clamp(value,min,max int) int {
    if value < min {
        return min
    }
    if value > max {
        return max
    }
    return value
}

func distort(x,y,w,h int, amp,freq,phase float64) (int, int) {
    dx := y + int(amp * math.Sin(freq * float64(x) + phase))
    dy := x + int(amp * math.Sin(freq * float64(y) + phase))
	dx = clamp(dx, 0, w-1)
	dy = clamp(dy, 0, h-1)
    return dx,dy
}

func videoFxRoutine(
    videoInName,framesDirName,vidOutName,imgType,expressionRed,expressionGreen,expressionBlue,expressionAlpha string,
    interpolationRatio,interpolationAdjust,reverbSampleRate,reverbLengthMs,reverbDecayFactor,reverbDamping,distAmp,distFreq,distPhase float64,
    useImageReverb,applyDistort,applyDct,dctBefore,applyDst,dstBefore bool) {
    if imgType != PNG && imgType != JPEG {
        log.Fatalf("videoFxRoutine(): ERROR :: Entered parameter imgType = %s is not .png or .bmp; please chose either .png or .jpg", imgType)
    }
    preRoutineCheck(videoInName,framesDirName)
    vidInFullPath := VIDIN+"/"+videoInName
    framesFullOutPath := IMGOUT+"/"+framesDirName
    teardownCommand := exec.Command(
        "ffmpeg", "-i", vidInFullPath,
        "-vf", "fps=30", framesFullOutPath+"/"+framesDirName+"_%03d."+imgType,
    )
    teardownOutput, err := teardownCommand.CombinedOutput()
    if err != nil {
        log.Fatalf("videoFxRoutine(): ERROR :: An error occured while running teardownCommand ->\n\n%s\n(%v)", string(teardownOutput), err)
    }
    fmt.Printf("\nvideoFxRoutine(): teardownCommand Output ->\n\n%s\n(Successfully created frames from source video %s in output directory %s)\n", string(teardownOutput), vidInFullPath, framesFullOutPath)
    framesFnames, err := ioutil.ReadDir(framesFullOutPath)
    if err != nil {
        log.Fatalf("videoFxRoutine(): ERROR :: An error occured while trying to read the names of frame files in %s: %v", framesFullOutPath, err)
    }
    var wg sync.WaitGroup
    var EXPR,EXPG,EXPB,EXPA interface {}
    var errR,errG,errB,errA error
    wg.Add(4)
    go func () {
        defer wg.Done()
        EXPR, errR = parser.ParseExpr(expressionRed)
    } ()
    go func () {
        defer wg.Done()
        EXPG, errG = parser.ParseExpr(expressionGreen)
    } ()
    go func () {
        defer wg.Done()
        EXPB, errB = parser.ParseExpr(expressionBlue)
    } ()
    go func () {
        defer wg.Done()
        EXPA, errA = parser.ParseExpr(expressionAlpha)
    } ()
    wg.Wait()
    if errR != nil {
        log.Fatalf("viedeoFxRoutine(): ERROR :: An error occured while parsing the expression %s for pixel Red color: %v", expressionRed, errR)
    }
    if errG != nil {
        log.Fatalf("viedeoFxRoutine(): ERROR :: An error occured while parsing the expression %s for pixel Green color: %v", expressionGreen, errG)
    }
    if errB != nil {
        log.Fatalf("viedeoFxRoutine(): ERROR :: An error occured while parsing the expression %s for pixel Blue color: %v", expressionBlue, errB)
    }
    if errA != nil {
        log.Fatalf("viedeoFxRoutine(): ERROR :: An error occured while parsing the expression %s for pixel Alpha color: %v", expressionAlpha, errA)
    }
    var interpRatio float64
    if interpolationRatio < 0.0 {
        interpRatio = 0.0
    } else if interpolationRatio > 1.0 {
        interpRatio = 1.0
    } else {
        interpRatio = interpolationRatio
    }
    nFilesFloat := float64(len(framesFnames))
    interpRatioAdj := interpolationAdjust / nFilesFloat
    for _, srcFrameFile := range framesFnames {
        frameFullName := framesFullOutPath+"/"+srcFrameFile.Name()
        rawFrameBytes, err := os.Open(frameFullName)
        if err != nil {
            log.Fatalf("videoFxRoutine(): ERROR :: An error occured while loading source frame bytes %s: %v", frameFullName, err)
        }
        frameImage, _, err := image.Decode(rawFrameBytes)
        if err != nil {
            log.Fatalf("videoFxRoutine(): ERROR :: An error occured while decoding source frame bytes %s: %v", frameFullName, err)
        }
        resultFrameRgba := image.NewRGBA(image.Rect(0, 0, frameImage.Bounds().Max.X, frameImage.Bounds().Max.Y))
        for y := 0; y < frameImage.Bounds().Max.Y; y++ {
            for x := 0; x < frameImage.Bounds().Max.X; x++ {
                dx, dy := x, y
                if applyDistort {
                    dx, dy = distort(x, y, frameImage.Bounds().Max.X, frameImage.Bounds().Max.Y, distAmp, distFreq, distPhase)
                }
                vars := map[string]int{ "x": dx, "y": dy }
                rTemp, err := evalExprTreeNode(EXPR, vars)
                if err != nil {
                    log.Fatalf("videoFxRoutine(): ERROR :: An error occured while evaluating parsed pixel Red expression %s for pixel coordinates x = %v, y = %v: %v", expressionRed, x, y, err)
                }
                rVal := uint8(rTemp)
                gTemp, err := evalExprTreeNode(EXPG, vars)
                if err != nil {
                    log.Fatalf("videoFxRoutine(): ERROR :: An error occured while evaluating parsed pixel Green expression %s for pixel coordinates x = %v, y = %v: %v", expressionGreen, x, y, err)
                }
                gVal := uint8(gTemp)
                bTemp, err := evalExprTreeNode(EXPB, vars)
                if err != nil {
                    log.Fatalf("videoFxRoutine(): ERROR :: An error occured while evaluating parsed pixel Blue expression %s for pixel coordinates x = %v, y = %v: %v", expressionBlue, x, y, err)
                }
                bVal := uint8(bTemp)
                aTemp, err := evalExprTreeNode(EXPA, vars)
                if err != nil {
                    log.Fatalf("videoFxRoutine(): ERROR :: An error occured while evaluating parsed pixel Alpha expression %s for pixel coordinates x = %v, y = %v: %v", expressionAlpha, x, y, err)
                }
                aVal := uint8(aTemp)
                rSrc,gSrc,bSrc,aSrc := frameImage.At(x, y).RGBA()
                resultFrameRgba.Set(x, y, color.RGBA{
                    uint8((interpRatio*float64(rSrc)) + ((1.0-interpRatio)*float64(rVal))), 
                    uint8((interpRatio*float64(gSrc)) + ((1.0-interpRatio)*float64(gVal))), 
                    uint8((interpRatio*float64(bSrc)) + ((1.0-interpRatio)*float64(bVal))),
                    uint8((interpRatio*float64(aSrc)) + ((1.0-interpRatio)*float64(aVal))),
                })
            }
        }
        if useImageReverb {
            resultFrameRgba = imageReverb(resultFrameRgba, reverbSampleRate, reverbLengthMs, reverbDecayFactor, reverbDamping)
        }
        segments := strings.Split(srcFrameFile.Name(), "_")
        idxStr := strings.Replace(segments[len(segments)-1], "."+imgType, "", -1)
        newFrameFname := fmt.Sprintf("%s/%s_fx_%s.%s", framesFullOutPath, framesDirName, idxStr, imgType)
        newFrameFile, err := os.Create(newFrameFname)
        if err != nil {
            log.Fatalf("videoFxRoutine(): ERROR :: An error occured while creating newFrameFile location %s/%s_fx_%s.%s: %v", framesFullOutPath, framesDirName, idxStr, imgType, err)
        }
        if strings.ToLower(imgType) == PNG {
            err = png.Encode(newFrameFile, resultFrameRgba)
        } else if strings.ToLower(imgType) == JPEG {
            err = jpeg.Encode(newFrameFile, resultFrameRgba, nil)
        }
        if err != nil {
            log.Fatalf("videoFxRoutine(): ERROR :: An error occured while saving newFrameFile %s/%s_fx_%s.%s: %v", framesFullOutPath, framesDirName, idxStr, imgType, err)
        }
        newFrameFile.Close()
        rawFrameBytes.Close()
        err = os.Remove(frameFullName)
        if err != nil {
            log.Fatalf("videoFxRoutine(): Error :: An error occurted while trying to clean up source frame %s/%s: %v", framesFullOutPath, srcFrameFile.Name(), err)
        }
        if interpRatio + interpRatioAdj > 1.0 {
            interpRatio = 1.0
        } else if interpRatio + interpRatioAdj < 0.0 {
            interpRatio = 0.0
        } else {
            interpRatio += interpRatioAdj
        }
    }
    recombineCommand := exec.Command(
        "ffmpeg", "-y",
        "-framerate", "30",
        "-i", framesFullOutPath+"/"+framesDirName+"_fx_%03d."+imgType,
        "-c:v", "libx264",
        "-pix_fmt", "yuv420p",
        VIDOUT+"/"+vidOutName,
    )
    recombineOutput, err := recombineCommand.CombinedOutput()
    if err != nil {
        log.Fatalf("videoFxRoutine(): An error occured while running recombineCommand ->\n\n%s\n(%v)", string(recombineOutput), err)
    }
    fmt.Println("\nvideoFxRoutine(): recombineCommand Output =>\n\n%s\n(Successfully created %s/%s from %s frames in %s)\n", string(recombineOutput), VIDOUT, vidOutName, imgType, framesFullOutPath)
}

func main() {
    videoFxRoutine(
        "test0.mp4", "test1", "test1.mp4", PNG, // videoInName, framesDirName, vidOutName, imgType
        //"sin(x+y)", "cos(x+y)", "tan(x+y)", "cos((x*sin(tan(x) + tan(y))) + (y*sin(tan(x) - tan(y)))", // expressionRed, expressionGreen, expressionBlue, expressionAlpha
        "255", "255", "255", "255", // testing
        1.0, 0.0, // interpolationRatio, interpolationAdjust,
        44100.0, 0.42, 0.69, 0.5, // reverbSampleRate, reverbLengthMs, reverbDecayFactor, reverbDamping
        0.0, 0.0, 0.0, // distAmp, distFreq, distPhase
        false, false, // useImageReverb, applyDistort
        false, false, // applyDct, dctBefore
        false, false, // applyDst, dstBefore
    )
}
