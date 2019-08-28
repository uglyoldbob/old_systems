module Things where

import qualified Data.Vector as V
import Data.Word
import Data.Bits as Bitwise

data NesState = NesState {
    cpu :: CpuState,
    cpuRam :: V.Vector Word8,
    stuff :: Word8} deriving Show

ramWrite st din addr = V.update st newval
 where newval = (V.singleton (addr, din))

ramRead st addr = (expandByte (st V.! addr))
       
nesPowerOn = NesState cpuPowerOn ramPowerOn 0
ramPowerOn = (V.replicate 2048 0)

stepNes sin = NesState nextCpu nextRam ((stuff sin) +1)
    where nextCpu = (cpuStep (CpuInputs One One One One 0) (cpu sin))
          nextRam = (cpuRam sin)

data Bit = One | Zero | HiZ | WeakOne | WeakZero | Invalid deriving Show

data CpuInstruction = Reset | NMI deriving Show

data CpuState = CpuState {
    clock_div :: Word8,
    instr_index :: Word8,
    instr :: CpuInstruction,
    a :: Word8,
    x :: Word8,
    y :: Word8,
    s :: Word8,
    p :: Word8,
    pc :: Word16,
    addr :: Word16} deriving Show

data CpuInputs = CpuInputs {
    reset :: Bit,
    nmi :: Bit,
    irq :: Bit,
    test :: Bit,
    din :: Word8}

firstCpuSteps x = mapM_ print (take x bla2)

everyN n xs = y : everyN n ys
    where y : ys = drop (n-1) xs

bla2 = everyN 12 bla
bla = cpuPowerOn : map (cpuStep (CpuInputs One One One One 0)) bla

cpuStep :: CpuInputs -> CpuState -> CpuState
cpuStep (CpuInputs Zero _ _ _ _) st = cpuReset st
cpuStep (CpuInputs One nmi irq test din) (CpuState cd ii i a x y s p pc addr) =
    case cd of
        0 -> CpuState 1 ii i a x y s p pc addr
        1 -> CpuState 2 ii i a x y s p pc addr
        2 -> CpuState 3 ii i a x y s p pc addr
        3 -> CpuState 4 ii i a x y s p pc addr
        4 -> CpuState 5 ii i a x y s p pc addr
        5 -> CpuState 6 ii i a x y s p pc addr
        6 -> CpuState 7 ii i a x y s p pc addr
        7 -> CpuState 8 ii i a x y s p pc addr
        8 -> CpuState 9 ii i a x y s p pc addr
        9 -> CpuState 10 ii i a x y s p pc addr
        10 -> CpuState 11 ii i a x y s p pc addr
        11 -> cpuDoSomething (CpuInputs One nmi irq test din) (CpuState 0 ii i a x y s p pc addr)

cpuDoSomething (CpuInputs One nmi irq test din) (CpuState cd ii i a x y s p pc addr) =
    case i of
        Reset -> case ii of
            0 -> CpuState cd 1 Reset a x y s p pc addr
            1 -> CpuState cd 2 Reset a x y s p pc (addr+1)
            2 -> CpuState cd 3 Reset a x y s p pc ((fromIntegral s)+256)
            3 -> CpuState cd 4 Reset a x y s p pc ((fromIntegral s)+255)
            4 -> CpuState cd 5 Reset a x y s p pc ((fromIntegral s)+254)
            5 -> CpuState cd 6 Reset a x y s p pc 0xfffc
            6 -> CpuState cd 0 Reset a x y s p pc 0xfffd
            otherwise -> CpuState cd 0 Reset a x y s p pc addr
        otherwise -> CpuState cd ii i a x y s (p-1) 5 addr

cpuPowerOn = CpuState 1 0 Reset 0 0 0 0xFD 0x34 0xFFFE 0
cpuReset (CpuState cd ii i a x y s p pc addr) = CpuState cd ii Reset a x y (s-3) (p .|. 4) pc addr

hiZByte = [HiZ|x<-[1..8]]
pullUpByte = [WeakOne|x<-[1..8]]
pullDownByte = [WeakZero|x<-[1..8]]

makeBit x = if (x > 0) then One else Zero

expandByte inp = map (makeBit) $ zipWith (.&.) (take 8 (repeat inp)) [2^x|x<-[0..]]

resolveByte x y = zipWith resolveBit x y
resolveBytes b = foldl (resolveByte) hiZByte b

collapseByte x = sum weightedBits
    where bits = map collapseBit x
          bitWeight = [2^x|x<-[0..]]
          weightedBits = zipWith (*) bits bitWeight

collapseBit x = case x of
    One -> 1
    Zero -> 0
    HiZ -> 1
    WeakOne -> 1
    WeakZero -> 0
    Invalid -> 1

resolveBit x y = case x of
    One -> case y of
        One -> One
        Zero -> Invalid
        HiZ -> One
        WeakOne -> One
        WeakZero -> One
        Invalid -> Invalid
    Zero -> case y of
        One -> Invalid
        Zero -> Zero
        HiZ -> Zero
        WeakOne -> Zero
        WeakZero -> Zero
        Invalid -> Invalid
    HiZ -> case y of
        One -> One
        Zero -> Zero
        HiZ -> HiZ
        WeakOne -> WeakOne
        WeakZero -> WeakZero
        Invalid -> Invalid
    WeakOne -> case y of
        One -> One
        Zero -> Zero
        HiZ -> WeakOne
        WeakOne -> WeakOne
        WeakZero -> HiZ --not sure
        Invalid -> Invalid
    WeakZero -> case y of
        One -> One
        Zero -> Zero
        HiZ -> WeakZero
        WeakOne -> HiZ --not sure
        WeakZero -> WeakZero
        Invalid -> Invalid
    Invalid -> Invalid

data Registers = Registers {
    r1 :: Word8,
    r2 :: Word8,
    r3 :: Word8 }