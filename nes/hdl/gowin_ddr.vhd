--
--Written by GowinSynthesis
--Tool Version "V1.9.10.03"
--Fri Nov 15 11:01:26 2024

--Source file index table:
--file0 "\/home/thomas/gowin/IDE/ipcore/DDR/data/ddr.v"
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
library gw2a;
use gw2a.components.all;

entity Gowin_DDR is
port(
  din :  in std_logic_vector(9 downto 0);
  fclk :  in std_logic;
  pclk :  in std_logic;
  reset :  in std_logic;
  q :  out std_logic_vector(0 downto 0));
end Gowin_DDR;
architecture beh of Gowin_DDR is
  signal VCC_0 : std_logic ;
  signal GND_0 : std_logic ;
begin
\oser10_gen[0].oser10_inst\: OSER10
port map (
  Q => q(0),
  D0 => din(0),
  D1 => din(1),
  D2 => din(2),
  D3 => din(3),
  D4 => din(4),
  D5 => din(5),
  D6 => din(6),
  D7 => din(7),
  D8 => din(8),
  D9 => din(9),
  PCLK => pclk,
  FCLK => fclk,
  RESET => reset);
VCC_s0: VCC
port map (
  V => VCC_0);
GND_s0: GND
port map (
  G => GND_0);
GSR_0: GSR
port map (
  GSRI => VCC_0);
end beh;
