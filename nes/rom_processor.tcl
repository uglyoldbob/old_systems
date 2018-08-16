puts "asdf"
set myfile "nestest.nes"
set fsize [file size $myfile]
set fp [open $myfile r]
fconfigure $fp -translation binary
set header [read $fp 16]

#puts [format %02X $magic4]
#puts $magic4

set check [string index $header 0]
scan $check "%c" magic1
set check [string index $header 1]
scan $check "%c" magic2
set check [string index $header 2]
scan $check "%c" magic3
set check [string index $header 3]
scan $check "%c" magic4

if {$magic1 == 78 && $magic2 == 69 && $magic3==83 && $magic4==26} {
  puts "YAY YAY"
  
  set check [string index $header 4]
  scan $check "%c" prg_rom_size
  
  set check [string index $header 5]
  scan $check "%c" chr_rom_size
  
  set check [string index $header 6]
  scan $check "%c" flags6
  
  set check [string index $header 7]
  scan $check "%c" flags7
  
  set check [string index $header 8]
  scan $check "%c" prg_ram_size
  
  set check [string index $header 9]
  scan $check "%c" flags9
  
  set check [string index $header 10]
  scan $check "%c" flags10
  
  if {$flags6 & 4} {
    puts "Trainer found"
	set trainer [read $fp 512]
  }
  
  if {$prg_rom_size > 0} {
    set calclength [expr $prg_rom_size * 16384]
    puts "Reading prg rom data"
	set prg_rom [read $fp $calclength]
	puts [string length $prg_rom]
  }
  
  if {$chr_rom_size > 0} {
    set calclength [expr $chr_rom_size * 8192]
    puts "Reading chr rom data"
	set chr_rom [read $fp $calclength]
	puts [string length $chr_rom]
  }
  
  
#  for {set i 0} {$i < 16} {incr i} {
#	set c [string index $header $i]
#	scan $c "%c" asciiValue
#	puts [format %02X $asciiValue]
#  }
} \
else {
  puts "Not an Ines rom"
}

close $fp