module Things where

import Data.Word
import Data.Bits as Bitwise

data Bit = One | Zero | HiZ | WeakOne | WeakZero | Invalid deriving Show

data CpuState = CpuState {
    clock_div :: Word8,
    instr_index :: Word8,
    a :: Word8,
    x :: Word8,
    y :: Word8,
    s :: Word8,
    p :: Word8,
    pc :: Word16} deriving Show

data CpuInputs = CpuInputs {
    reset :: Bit,
    nmi :: Bit,
    irq :: Bit,
    test :: Bit}

data IoByte = IoByte Word32 deriving Show

bla = cpuPowerOn : map (cpuStep (CpuInputs One One One One)) bla

cpuStep :: CpuInputs -> CpuState -> CpuState
cpuStep (CpuInputs Zero _ _ _) st = cpuReset st
cpuStep (CpuInputs One nmi irq test) (CpuState cd ii a x y s p pc) =
    case cd of
        0 -> CpuState 1 ii a x y s p pc
        1 -> CpuState 2 ii a x y s p pc
        2 -> CpuState 3 ii a x y s p pc
        3 -> CpuState 4 ii a x y s p pc
        4 -> CpuState 5 ii a x y s p pc
        5 -> CpuState 6 ii a x y s p pc
        6 -> CpuState 7 ii a x y s p pc
        7 -> CpuState 8 ii a x y s p pc
        8 -> CpuState 9 ii a x y s p pc
        9 -> CpuState 10 ii a x y s p pc
        10 -> CpuState 11 ii a x y s p pc
        11 -> cpuDoSomething (CpuInputs One nmi irq test) (CpuState 0 ii a x y s p pc)

cpuDoSomething (CpuInputs One nmi irq test) (CpuState cd ii a x y s p pc) =
    CpuState cd ii a x y s (p-1) 5

cpuPowerOn = CpuState 0 0 0 0 0 0xFD 0x34 0xFFFE
cpuReset (CpuState cd ii a x y s p pc) = CpuState cd ii a x y (s-3) (p .|. 4) pc

hiZByte = [HiZ|x<-[1..8]]
pullUpByte = [WeakOne|x<-[1..8]]
pullDownByte = [WeakZero|x<-[1..8]]

resolveByte x y = zipWith resolveBit x y
resolveBytes b = foldl (resolveByte) hiZByte b

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

resolve (IoByte x) (IoByte y) = IoByte (x + y)

combine :: [IoByte] -> IoByte
combine x = foldl (resolve) (IoByte 0) x

data Registers = Registers {
    r1 :: Word8,
    r2 :: Word8,
    r3 :: Word8 }