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
  
  set opf [open "test.vhd" "w"]
  puts $opf "library IEEE;"
  puts $opf "use IEEE.STD_LOGIC_1164.ALL;"
  puts $opf "use IEEE.NUMERIC_STD.ALL;"
  puts $opf "entity nes_cartridge is"
  puts $opf "    Port (cic_out: out std_logic;"
  puts $opf "			cic_in: in std_logic;"
  puts $opf "			cic_clk: in std_logic;"
  puts $opf "			cic_rst: in std_logic;"
  puts $opf "			ppu_data: inout std_logic_vector(7 downto 0);"
  puts $opf "			ppu_addr: in std_logic_vector(13 downto 0);"
  puts $opf "			ppu_addr_13: in std_logic;"
  puts $opf "			ppu_wr: in std_logic;"
  puts $opf "			ppu_rd: in std_logic;"
  puts $opf "			ciram_a10: out std_logic;"
  puts $opf "			ciram_ce: out std_logic;"
  puts $opf "			exp: inout std_logic_vector(9 downto 0);"
  puts $opf "			irq: out std_logic;"
  puts $opf "			cpu_rw: in std_logic;"
  puts $opf "			romsel: in std_logic;"
  puts $opf "			cpu_data: inout std_logic_vector(7 downto 0);"
  puts $opf "			cpu_addr: in std_logic_vector(14 downto 0);"
  puts $opf "			m2: in std_logic;"
  puts $opf "			clock: in std_logic);"
  puts $opf "end nes_cartridge;"
  puts $opf "architecture Behavioral of nes_cartridge is"
  puts $opf "	signal recovered_address: std_logic_vector(15 downto 0);"
  puts $opf "begin"
  puts $opf "	recovered_address(14 downto 0) <= cpu_addr;"
  puts $opf "	recovered_address(15) <= not romsel;"
  puts $opf "	prg_rom: entity work.sram_init "
  puts $opf "		generic map (num_bits => 15, filename => \"rom.txt\")"
  puts $opf "		port map("
  puts $opf "			addr => cpu_addr(14 downto 0),"
  puts $opf "			oe => not cpu_rw,"
  puts $opf "			we => cpu_rw,"
  puts $opf "			cs => romsel,"
  puts $opf "			data => cpu_data);"
  puts $opf "	chr_rom: entity work.sram_init"
  puts $opf "		generic map (num_bits => 13, filename => \"chr_rom.txt\")"
  puts $opf "		port map("
  puts $opf "			addr => (others => '0'),"
  puts $opf "			oe => '1',"
  puts $opf "			we => '1',"
  puts $opf "			cs => '1',"
  puts $opf "			data => ppu_data);"
  puts $opf "	ctg_ram: entity work.sram"
  puts $opf "		generic map (num_bits => 12)"
  puts $opf "		port map("
  puts $opf "			addr => (others => '0'),"
  puts $opf "			oe => '1',"
  puts $opf "			we => '1',"
  puts $opf "			cs => '1',"
  puts $opf "			data => cpu_data);"
  puts $opf "end Behavioral;"
  close $opf
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