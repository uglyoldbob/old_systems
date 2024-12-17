library IEEE;
use ieee.std_logic_1164.all;

use IEEE.NUMERIC_STD.ALL;

entity clock_divider is
	Port (
		pause_cpu: in std_logic;
		reset: in std_logic;
		clock: in std_logic;
		c1: out std_logic;
		c2: out std_logic;
		c3: out std_logic;
		c4: out std_logic;
		c5: out std_logic);
end clock_divider;

architecture Behavioral of clock_divider is
	signal counter: integer range 0 to 7 := 0;
	signal counter2: integer range 0 to 7 := 0;
	signal clocko: std_logic := '0';
	signal clocko_n: std_logic := '1';
	signal clocko2: std_logic := '0';
begin
	c1 <= clocko;
	c5 <= clocko_n;
	c2 <= clocko2;

	process (reset, clock)
	begin
		if rising_edge(clock) then
			if reset='1' then
				counter <= 0;
				counter2 <= 0;
				c4 <= '0';
				clocko <= '0';
				clocko_n <= '1';
				clocko2 <= '1';
			elsif pause_cpu = '0' then
				c3 <= clocko;
				counter <= counter + 1;
				counter2 <= counter2 + 1;
				if counter = 5 then
					counter <= 0;
					clocko <= not clocko;
					clocko_n <= not clocko_n;
					clocko2 <= not clocko2;
				end if;
				if counter2 = 1 then
					counter2 <= 0;
					c4 <= not c4;
				end if;
			end if;
		end if;
	end process;
end Behavioral;

library IEEE;
use ieee.std_logic_1164.all;

use IEEE.NUMERIC_STD.ALL;

entity nes_cpu is
	Generic (
		ramtype: string := "sram");
   Port (pause_cpu: in std_logic;
			clock : in STD_LOGIC;
         audio : out STD_LOGIC_VECTOR (1 downto 0);
         address : out STD_LOGIC_VECTOR (15 downto 0);
			memory_start: out std_logic;
			memory_clock: out std_logic;
			ppu_clock: out std_logic;
			memory_cycle_done: in std_logic;
         dout : out STD_LOGIC_VECTOR (7 downto 0);
			din: in STD_LOGIC_VECTOR (7 downto 0);
         cout : out STD_LOGIC_VECTOR (2 downto 0);
			oe: out STD_LOGIC_VECTOR (1 downto 0);
         rw : out STD_LOGIC;
         nmi : in STD_LOGIC;
         irq : in STD_LOGIC;
         m2 : out STD_LOGIC;
         tst : in STD_LOGIC;
			d_a: out std_logic_vector(7 downto 0) := x"00";
			d_x: out std_logic_vector(7 downto 0) := x"00";
			d_y: out std_logic_vector(7 downto 0) := x"00";
			d_pc: out std_logic_vector(15 downto 0);
			d_sp: out std_logic_vector(7 downto 0) := x"fd";
			d_flags: out std_logic_vector(7 downto 0) := x"24";
			d_cycle: out std_logic_vector(14 downto 0);
			d_subcycle: out std_logic_vector(3 downto 0);
			instruction_toggle_out: out std_logic;
         reset : in STD_LOGIC);
end nes_cpu;

architecture Behavioral of nes_cpu is
	signal cycle_counter: std_logic_vector(14 downto 0);

	signal clocka: std_logic;
	signal clockb: std_logic;
	signal clockm: std_logic;

	signal instruction_toggle: std_logic;
	signal cycle_toggle: std_logic;
	
	signal pc: std_logic_vector(15 downto 0);
	signal next_pc: std_logic_vector(15 downto 0);
	
	signal a: std_logic_vector(7 downto 0) := x"00";
	signal x: std_logic_vector(7 downto 0) := x"00";
	signal y: std_logic_vector(7 downto 0) := x"00";
	signal sp: std_logic_vector(7 downto 0) := x"00";
	signal flags: std_logic_vector(7 downto 0) := x"24";
	signal next_dout: std_logic_vector(7 downto 0);
	
	constant FLAG_CARRY: integer := 0;
	constant FLAG_ZERO: integer := 1;
	constant FLAG_INTERRUPT: integer := 2;
	constant FLAG_DECIMAL: integer := 3;
	constant FLAG_BREAK: integer := 4;
	constant FLAG_UNUSED: integer := 5;
	constant FLAG_OVERFLOW: integer := 6;
	constant FLAG_NEGATIVE: integer := 7;
	
	signal reset_active: std_logic; --Indicates that the cpu is in reset
	signal opcode: std_logic_vector(8 downto 0); -- bit 8 indicates the opcode is valid
	signal op_byte_2: std_logic_vector(7 downto 0);
	signal op_byte_3: std_logic_vector(7 downto 0);
	signal dma_cycle: std_logic;
	
	signal subcycle: std_logic_vector(3 downto 0);
	signal oe1: std_logic;
	signal oe2: std_logic;
	
	signal dma_running: std_logic;
	signal dma_dmc: std_logic_vector(16 downto 0);
	signal dma_oam: std_logic_vector(8 downto 0);
	signal dma_count: std_logic_vector(8 downto 0);
	signal dma_dmc_counter: std_logic_vector(7 downto 0);
	
	signal sp_plus: std_logic_vector(7 downto 0);
	signal sp_minus: std_logic_vector(7 downto 0);
	
	signal rw_address: std_logic_vector(15 downto 0);
	signal read_cycle: std_logic; --indicates a read cycle when true
	signal stall: std_logic;
	signal stall_clocked: std_logic;
	
	signal instruction_length: std_logic_vector(3 downto 0);
	signal pre_execution_length: std_logic_vector(3 downto 0);
	signal done_fetching: std_logic;
	signal ready_to_execute: std_logic;
	signal execution_cycle: std_logic_vector(3 downto 0);
	signal early_execute: std_logic;
	signal end_fetch: std_logic;
	signal pc_increment: std_logic;
	
	signal instruction_toggle_pre: std_logic;
	
	signal addr_calc1: std_logic_vector(15 downto 0);
	signal addr_calc2: std_logic_vector(15 downto 0);
	signal indirect_y_addr: std_logic_vector(8 downto 0);
	signal absolute_y_addr: std_logic_vector(8 downto 0);
	signal absolute_x_addr: std_logic_vector(8 downto 0);
	
	signal sub_in: std_logic_vector(7 downto 0);
	signal sub: std_logic_vector(7 downto 0);
	signal subm: std_logic_vector(7 downto 0);

	signal adc_carry_in: std_logic_vector(8 downto 0);
	signal adc_result: std_logic_vector(8 downto 0);
	signal adc_overflow: std_logic;
	signal adcm_result: std_logic_vector(8 downto 0);
	signal adcm_overflow: std_logic;
	
	signal sbc_carry_in: std_logic_vector(8 downto 0);
	signal sbc_result: std_logic_vector(8 downto 0);
	signal sbcm_result: std_logic_vector(8 downto 0);
	signal sbc_overflow: std_logic;
	signal sbcm_overflow: std_logic;
	
	signal inc_out: std_logic_vector(7 downto 0);
	
	signal flag_check: std_logic;
	
	signal op_byte_2_plus_x: std_logic_vector(7 downto 0);
	signal op_byte_2_plus_y: std_logic_vector(7 downto 0);
	signal op_byte_2_plus_one: std_logic_vector(7 downto 0);
	
	signal extra_cycle: std_logic_vector(3 downto 0) := (others => '0');
begin

	d_a <= a;
	d_x <= x;
	d_y <= y;
	d_pc <= pc;
	d_sp <= sp;
	d_flags <= flags;
	d_subcycle <= subcycle;
	d_cycle <= cycle_counter;

	clockd: entity work.clock_divider port map (
		pause_cpu => pause_cpu,
		reset => reset,
		clock => clock,
		c1 => clocka,
		c3 => clockm,
		c4 => ppu_clock,
		c5 => clockb);

	memory_clock <= clockm;
	
	process (all)
	begin
		case opcode(7 downto 5) is
			when "000" => flag_check <= flags(FLAG_NEGATIVE);
			when "001" => flag_check <= not flags(FLAG_NEGATIVE);
			when "010" => flag_check <= flags(FLAG_OVERFLOW);
			when "011" => flag_check <= not flags(FLAG_OVERFLOW);
			when "100" => flag_check <= flags(FLAG_CARRY);
			when "101" => flag_check <= not flags(FLAG_CARRY);
			when "110" => flag_check <= flags(FLAG_ZERO);
			when others => flag_check <= not flags(FLAG_ZERO);
		end case;
	end process;

   process (rw_address)
	begin
		if rw_address = x"4016" then
			oe1 <= '0';
		else
			oe1 <= '1';
		end if;
	end process;

	process (rw_address)
	begin
		if rw_address = x"4017" then
			oe2 <= '0';
		else
			oe2 <= '1';
		end if;
	end process;
	oe <= oe1 & oe2;
	
	process (all)
	begin
		if read_cycle = '1' then
			stall <= '0';
			address <= rw_address;
			rw <= '1';
		else
			stall <= '0';
			rw <= '0';
			address <= rw_address;
		end if;
	end process;
	
	process (all)
	begin
		adc_carry_in <= (0 => flags(FLAG_CARRY), others=>'0');
		adc_result <= std_logic_vector(unsigned('0' & a) + unsigned('0' & din) + unsigned(adc_carry_in));
		adc_overflow <= (a(7) xor adc_result(7)) and (din(7) xor adc_result(7));
		adcm_result <= std_logic_vector(unsigned('0' & a) + unsigned('0' & next_dout) + unsigned(adc_carry_in));
		adcm_overflow <= (a(7) xor adcm_result(7)) and (next_dout(7) xor adcm_result(7));
		
		sbc_carry_in <= (0 => not flags(FLAG_CARRY), others=>'0');
		sbc_result <= std_logic_vector(unsigned('0' & a) - unsigned('0' & din) - unsigned(sbc_carry_in));
		sbcm_result <= std_logic_vector(unsigned('0' & a) - unsigned('0' & next_dout) - unsigned(sbc_carry_in));
		sbc_overflow <= (a(7) xor sbc_result(7)) and (din(7) xor sbc_result(7) xor '1');
		sbcm_overflow <= (a(7) xor sbcm_result(7)) and (next_dout(7) xor sbcm_result(7) xor '1');
		
		sp_plus <= std_logic_vector(unsigned(sp) + "00000001");
		sp_minus <= std_logic_vector(unsigned(sp) - "00000001");
		
		op_byte_2_plus_x <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
		op_byte_2_plus_y <= std_logic_vector(unsigned(op_byte_2) + unsigned(y));
		op_byte_2_plus_one <= std_logic_vector(unsigned(op_byte_2) + 1);
	end process;
	
	process (all)
	begin
		case opcode(7 downto 0) is
			when x"c0" | x"c4" | x"cc" => sub_in <= y;
			when x"e0" | x"e4" | x"ec" => sub_in <= x;
			when others => sub_in <= a;
		end case;
		sub <= std_logic_vector(unsigned(sub_in) - unsigned(din));
		subm <= std_logic_vector(unsigned(sub_in) - unsigned(next_dout));
		case opcode(7 downto 0) is
			when x"88" => inc_out <= std_logic_vector(unsigned(y) - "00000001");
			when x"c8" => inc_out <= std_logic_vector(unsigned(y) + "00000001");
			when x"ca" => inc_out <= std_logic_vector(unsigned(x) - "00000001");
			when others => inc_out <= std_logic_vector(unsigned(x) + "00000001");
		end case;
	end process;

	process (all)
	begin
		if pc_increment = '1' then
			next_pc <= std_logic_vector(unsigned(pc(15 downto 0)) + "0000000000000001");
		else
			next_pc <= pc;
		end if;
	end process;
	
	process(all)
	begin
		indirect_y_addr <= std_logic_vector(unsigned('0' & op_byte_3) + unsigned('0' & y));
		absolute_y_addr <= std_logic_vector(unsigned('0' & op_byte_2) + unsigned('0' & y));
		absolute_x_addr <= std_logic_vector(unsigned('0' & op_byte_2) + unsigned('0' & x));
	end process;
	
	process (clock)
	begin
		if rising_edge(clock) then
			memory_start <= (clocka xor clockm) and clocka and not clockm;
		end if;
	end process;
	
	process (reset, clocka)
	begin
		if reset='1' then
			read_cycle <= '1';
			cycle_toggle <= '0';
			cycle_counter <= (others => '0');
		elsif rising_edge(clocka) then
			dma_cycle <= NOT dma_cycle;
			cycle_toggle <= not cycle_toggle;
			instruction_toggle_out <= instruction_toggle_pre;
			instruction_toggle <= instruction_toggle_pre;
			cycle_counter <= std_logic_vector(unsigned(cycle_counter) + 1);
			read_cycle <= '1';
			rw_address <= pc;
			if reset_active then
				case subcycle is
					when "0000" => null;
					when "0001" => null;
					when "0010" =>
						rw_address <= x"01" & sp;
					when "0011" =>
						rw_address <= x"01" & sp;
					when "0100" =>
						rw_address <= x"01" & sp;
					when "0101" =>
						rw_address <= x"FFFC";
					when "0110" =>
						rw_address <= x"FFFD";
					when others =>
					   rw_address <= "XXXXXXXXXXXXXXXX";
				end case;
			else
				dout <= next_dout;
				if opcode(8) and done_fetching then
					case opcode(7 downto 0) is
						when x"20" =>
							case subcycle(2 downto 0) is
								when "011" | "100" =>
									rw_address <= x"01" & sp;
									read_cycle <= '0';
								when others => null;
							end case;
						when x"40" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when others =>
									rw_address <= x"01" & sp;
							end case;
						when x"60" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= x"01" & sp;
								when "011" =>
									rw_address <= x"01" & sp;
								when others => null;
							end case;
						when x"08" | x"48" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when others =>
									rw_address <= x"01" & sp;
									read_cycle <= '0';
							end case;
						when x"28" | x"68" =>
							rw_address <= x"01" & sp;
						--nop indirect x
						when x"14" | x"34" | x"54" | x"74" | x"d4" | x"f4" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when others =>
									rw_address <= x"00" & op_byte_2; --not technically correct for last cycle, doesn't matter for zero page
							end case;
						--indirect x
						when x"01" | x"21" | x"41" | x"61" | x"a1" | x"a3" | x"c1" | x"e1" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" | "011" =>
									rw_address <= x"00" & op_byte_2;
								when "100" =>
									rw_address <= x"00" & op_byte_2_plus_one;
								when others =>
									rw_address <= addr_calc2;
							end case;
						--indirect x store
						when x"81" | x"83" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" | "011" =>
									rw_address <= x"00" & op_byte_2;
								when "100" =>
									rw_address <= x"00" & op_byte_2_plus_one;
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
							end case;
						--indirect x/y rmw
						when x"03" | x"13" | x"23" | x"33" | x"43" | x"53" | x"63" | x"73" | x"c3" | x"e3" | x"f3" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" =>
									rw_address <= x"00" & op_byte_2;
								when "011" =>
									rw_address <= x"00" & op_byte_2_plus_one;
								when "100" | "101" =>
									rw_address <= addr_calc2;
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
							end case;
						--indirect y write dcp
						when x"d3" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" =>
									rw_address <= x"00" & din;
								when "011" =>
									rw_address <= x"00" & op_byte_2_plus_one;
								when "100" | "101" =>
									rw_address <= addr_calc2;
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
							end case;
						--indirect y write
						when x"91" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" =>
									rw_address <= x"00" & din;
								when "011" =>
									rw_address <= x"00" & op_byte_2_plus_one;
								when "100" =>
									rw_address <= addr_calc2;
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
							end case;
						--indirect y read
						when x"11" | x"31" | x"51" | x"71" | x"b1" | x"b3" | x"d1" | x"f1" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" =>
									rw_address <= x"00" & din;
								when "011" =>
									rw_address <= x"00" & op_byte_2_plus_one;
								when others =>
									rw_address <= addr_calc2;
							end case;
						--indirect jmp
						when x"6c" =>
							case subcycle(2 downto 0) is
								when "011" =>
									rw_address <= op_byte_3 & op_byte_2;
								when "100" =>
									rw_address <= op_byte_3 & op_byte_2_plus_one;
								when others => null;
							end case;
						--zero page y write
						when x"96" | x"97" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" =>
									rw_address <= x"00" & din;
								when others =>
									rw_address <= x"00" & op_byte_2_plus_y;
									read_cycle <= '0';
							end case;
						--zero page x write
						when x"94" | x"95" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" =>
									rw_address <= x"00" & din;
								when others =>
									rw_address <= x"00" & op_byte_2_plus_x;
									read_cycle <= '0';
							end case;
						--zero page y
						when x"b6" | x"b7" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" =>
									rw_address <= x"00" & din;
								when others =>
									rw_address <= x"00" & op_byte_2_plus_y;
							end case;
						--zero page x
						when x"15" | x"35" | x"55" | x"75" | x"b4" | x"b5" | x"d5" | x"f5" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" =>
									rw_address <= x"00" & din;
								when others =>
									rw_address <= x"00" & op_byte_2_plus_x;
							end case;
						--zero page read
						when x"04" | x"24" | x"44" | x"64" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when others =>
									rw_address <= x"00" & din;
							end case;
						--zero page read
						when x"05" | x"25" | x"45" | x"65" | x"a4" | x"a5" | x"a6" | x"a7" | x"c4" | x"c5" | x"e4" | x"e5" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when others =>
									rw_address <= x"00" & op_byte_2;
							end case;
						--zero page x rmw
						when x"16" | x"17" | x"36" | x"37" | x"56" | x"57" | x"76" | x"77" | x"d6" | x"d7" | x"f6" | x"f7" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when "010" =>
									rw_address <= x"00" & din;
								when "011" =>
									rw_address <= x"00" & op_byte_2_plus_x;
								when others =>
									rw_address <= x"00" & op_byte_2_plus_x;
									read_cycle <= '0';
							end case;
						--zero page rmw
						when x"06" | x"07" | x"26" | x"27" | x"46" | x"47" | x"66" | x"67" | x"c6" | x"c7" | x"e6" | x"e7" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= x"00" & op_byte_2;
								when "011" =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '0';
									dout <= din;
								when "100" =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '0';
								when others => null;
							end case;
						--zero page write
						when x"84" | x"85" | x"86" | x"87" =>
							case subcycle(2 downto 0) is
								when "001" => null;
								when others =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '0';
							end case;
						--absolute write a/x/y
						when x"8c" | x"8d" | x"8e" | x"8f" =>
							case subcycle(2 downto 0) is
								when "010" => null;
								when others =>
									rw_address <= op_byte_3 & op_byte_2;
									read_cycle <= '0';
							end case;
						--absolute rmw
						when x"0e" | x"0f" | x"2e" | x"2f" | x"4e" | x"4f" | x"6e" | x"6f" | x"ce" | x"cf" | x"ee" | x"ef" =>
							case subcycle(2 downto 0) is
								when "010" => null;
								when "011" =>
									rw_address <= op_byte_3 & op_byte_2;
								when "100" =>
									read_cycle <= '0';
									rw_address <= op_byte_3 & op_byte_2;
									dout <= din;
								when others =>
									read_cycle <= '0';
									rw_address <= op_byte_3 & op_byte_2;
							end case;
						--absolute read
						when	x"0c" | x"0d" | x"2c" | x"2d" | x"4d" | x"6d" | x"ac" | x"ad" | 
								x"ae" | x"af" | x"cc" | x"cd" | x"ec" | x"ed" =>
							case subcycle(2 downto 0) is
								when "010" => null;
								when others =>
									rw_address <= op_byte_3 & op_byte_2;
							end case;
						--absolute y write
						when x"99" =>
							case subcycle(2 downto 0) is
								when "010" => null;
								when "011" =>
									rw_address <= din & op_byte_2_plus_y;
								when others =>
									rw_address <=  std_logic_vector(unsigned(op_byte_3 & op_byte_2) + unsigned(x"00" & y));
									read_cycle <= '0';
							end case;
						--absolute x/y rmw
						when	x"1b" | x"1e" | x"1f" | x"3b" | x"3e" | x"3f" | x"5b" | x"5e" | 
								x"5f" | x"7b" | x"7e" | x"7f" | x"db" | x"de" | x"df" | x"fb" | 
								x"fe" | x"ff" =>
							case subcycle(2 downto 0) is
								when "010" => null;
								when "011" | "100" =>
									rw_address <= addr_calc2;
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
							end case;
						--absolute x/y write
						when x"9d" =>
							case subcycle(2 downto 0) is
								when "010" => null;
								when "011" =>
									rw_address <= addr_calc2;
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
							end case;
						--absolute x/y read
						when	x"19" | x"1c" | x"1d" | x"39" | x"3c" | x"3d" | x"59" | x"5c" | 
								x"5d" | x"79" | x"7c" | x"7d" | x"b9" | x"bc" | x"bd" | x"be" | 
								x"bf" | x"d9" | x"dc" | x"dd" | x"f9" | x"fc" | x"fd" =>
							case subcycle(2 downto 0) is
								when "010" => null;
								when others =>
									rw_address <= addr_calc2;
							end case;
						when others => null;
					end case;
				end if;
			end if;
		end if;
	end process;
	
	process (all)
	begin
		case opcode(7 downto 0) is
			when	x"01" | x"03" | x"04" | x"05" | x"06" | x"07" | x"09" | x"10" | 
					x"11" | x"13" | x"14" | x"15" | x"16" | x"17" | x"21" | x"23" | x"24" | 
					x"25" | x"26" | x"27" | x"29" | x"30" | x"31" | x"33" | x"34" | x"35" | x"36" | 
					x"37" | x"41" | x"43" | x"44" | x"45" | x"46" | x"47" | x"49" | x"50" | x"51" | x"53" | x"54" | 
					x"55" | x"56" | x"57" | x"61" | x"63" | x"64" | x"65" | x"66" | x"67" | x"69" | x"70" | 
					x"71" | x"73" | x"74" | x"75" | x"76" | x"77" | x"80" | x"81" | x"83" | x"84" | 
					x"85" | x"86" | x"87" | x"90" | x"91" | x"94" | x"95" | x"96" | 
					x"97" | x"a0" | x"a1" | x"a2" | x"a3" | x"a4" | x"a5" | x"a6" | 
					x"a7" | x"a9" | x"b0" | x"b1" | x"b3" | x"b4" | x"b5" | x"b6" | 
					x"b7" | x"c0" | x"c1" | x"c3" | x"c4" | x"c5" | x"c6" | x"c7" | 
					x"c9" | x"d0" | x"d1" | x"d3" | x"d4" | x"d5" | x"d6" | x"d7" | 
					x"e0" | x"e1" | x"e3" | x"e4" | x"e5" | x"e6" | x"e7" | x"e9" | 
					x"eb" | x"f0" | x"f1" | x"f3" | x"f4" | x"f5" | x"f6" | x"f7" => instruction_length <= "0001";
			when	x"0c" | x"0d" | x"0e" | x"0f" | x"19" | x"1b" | x"1c" | x"1d" | 
					x"1e" | x"1f" | x"20" | x"2c" | x"2d" | x"2e" | x"2f" | x"39" | x"3b" | x"3c" | 
					x"3d" | x"3e" | x"3f" | x"4c" | x"4d" | x"4e" | x"4f" | x"59" | x"5b" | x"5c" | x"5d" | 
					x"5e" | x"5f" | x"6c" | x"6d" | x"6e" | x"6f" | x"79" | x"7b" | x"7c" | x"7d" | x"7e" | x"7f" |
					x"8c" | x"8d" | x"8e" | x"8f" | x"99" | x"9d" | x"ac" | x"ad" | 
					x"ae" | x"af" | x"b9" | x"bc" | x"bd" | x"be" | x"bf" | x"cc" | 
					x"cd" | x"ce" | x"cf" | x"d9" | x"db" | x"dc" | x"dd" | x"de" | 
					x"df" | x"ec" | x"ed" | x"ee" | x"ef" | x"f9" | x"fb" | x"fc" | 
					x"fd" | x"fe" | x"ff" => instruction_length <= "0010";
			when others => instruction_length <= "0000";
		end case;
		execution_cycle <= std_logic_vector(unsigned(subcycle) - unsigned(pre_execution_length));
		case opcode(7 downto 0) is
			when x"81" | x"83" | x"9d" => 
				if subcycle(2 downto 0) = "100" then
					early_execute <= '1';
				else
					early_execute <= '0';
				end if;
			when x"bf" => 
				if subcycle(2 downto 0) = "011" then
					early_execute <= '1';
				else
					early_execute <= '0';
				end if;
			when others => early_execute <= '0';
		end case;
		case opcode(7 downto 0) is
			when	x"01" | x"03" | x"1e" | x"21" | x"23" | x"3e" | x"41" | x"43" | 
					x"5e" | x"61" | x"63" | x"7e" | x"a1" | x"a3" | x"c1" | x"c3" | x"e1" | x"e3" =>
				pre_execution_length <= std_logic_vector(unsigned(extra_cycle) + "0101");
			when	x"0e" | x"11" | x"13" | x"16" | x"1c" | x"2e" | x"31" | x"33" | x"36" | x"3c" | 
					x"4e" | x"51" | x"53" | x"56" | x"5c" | x"6e" | x"71" | x"73" | x"76" |
					x"7c" | x"81" | x"83" | x"91" | x"b1" | x"b3" | x"bf" | x"d1" | x"d3" | x"dc" | x"de" | x"f1" | x"f3" | x"fc" | x"fe" =>
				pre_execution_length <= std_logic_vector(unsigned(extra_cycle) + "0100");
			when	x"06" | x"0c" | x"0d" | x"0f" | x"14" | x"15" | x"17" | x"19" | x"1b" | 
					x"1d" | x"1f" | x"26" | x"2c" | 
					x"2d" | x"2f" | x"34" | x"35" | x"37" | x"39" | x"3b" | x"3d" | x"3f" | 
					x"46" | x"4d" | x"4f" | x"54" | 
					x"55" | x"57" | x"59" | x"5b" | x"5d" | x"5f" | x"66" | x"6c" | x"6d" |
					x"6f" | x"74" | x"75" | x"77" | x"79" | x"7b" | x"7d" | x"7f" | x"99" | x"9d" | x"ac" | x"ad" | 
					x"ae" | x"af" | x"b4" | x"b5" | x"b6" | x"b7" | x"b9" | x"bc" | x"bd" | x"be" | x"cc" | x"cd" | x"ce" | x"cf" |
					x"d4" | x"d5" | x"d6" | x"d7" | x"d9" | x"db" | x"dd" | x"df" | x"ec" | 
					x"ed" | x"ee" | x"ef" | x"f4" | x"f5" | x"f6" | x"f7" | x"f9" | x"fb" | x"fd" | x"ff" =>
				pre_execution_length <= std_logic_vector(unsigned(extra_cycle) + "0011");
			when	x"04" | x"05" | x"07" | x"20" | x"24" | x"25" | x"27" | x"44" | 
					x"45" | x"47" | x"4c" | x"64" | x"65" | x"67" | x"8c" | x"8d" | 
					x"8e" | x"8f" | x"94" | x"95" | x"96" | x"97" |
					x"a4" | x"a5" | x"a6" | x"a7" | x"c4" | x"c5" | x"c6" | x"c7" | 
					x"e4" | x"e5" | x"e6" | x"e7" => 
				pre_execution_length <= std_logic_vector(unsigned(extra_cycle) + "0010");
			when others => pre_execution_length <= std_logic_vector(unsigned(extra_cycle) + "0001");
		end case;
		if subcycle >= pre_execution_length then
			ready_to_execute <= '1';
		else
			ready_to_execute <= '0';
		end if;
		if instruction_length <= subcycle(3 downto 0) then
			done_fetching <= '1';
		else
			done_fetching <= '0';
		end if;
		if instruction_length >= subcycle(3 downto 0) then
			pc_increment <= '1';
		else
			pc_increment <= '0';
		end if;
		if instruction_length(3 downto 0) = subcycle(3 downto 0) then
			end_fetch <= '1';
		else
			end_fetch <= '0';
		end if;
		addr_calc1 <= std_logic_vector(unsigned(next_pc(15 downto 0)) + unsigned(din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din));
	end process;
	
	process (clockm)
	begin
		if rising_edge(clockm) then
			stall_clocked <= stall;
		end if;
	end process;
	
	process(reset, clockb)
	begin
		if reset='1' then
			pc <= x"FFFC";
			flags(FLAG_INTERRUPT) <= '1';
			opcode <= "011111111";
			reset_active <= '1';
			subcycle <= "0000";
			instruction_toggle_pre <= '0';
		elsif rising_edge(clockb) then
			if reset_active then
				subcycle <= std_logic_vector(unsigned(subcycle(3 downto 0)) + "0001");
				pc <= next_pc;
				case subcycle(2 downto 0) is
					when "000" | "001" => null;
					when "010" | "011" | "100" =>
						sp <= sp_minus;
					when "101" =>
						pc <= pc(15 downto 8) & din;
					when others =>
						pc <= din & pc(7 downto 0);
						reset_active <= '0';
						instruction_toggle_pre <= not instruction_toggle_pre;
						subcycle <= (others => '0');
						opcode(8) <= '0';
				end case;
			elsif stall_clocked = '0' then
				subcycle <= std_logic_vector(unsigned(subcycle(3 downto 0)) + "0001");
				if opcode(8) = '0' then
					pc <= next_pc;
					opcode <= '1' & din;
					op_byte_2 <= x"00";
					op_byte_3 <= x"00";
				elsif done_fetching = '0' then
					pc <= next_pc;
					case subcycle is
						when "0001" => op_byte_2 <= din;
						when others => op_byte_3 <= din;
					end case;
				else
					case subcycle is
						when "0001" => op_byte_2 <= din;
						when "0010" => op_byte_3 <= din;
						when others =>
					end case;
					--todo remove all manual instances of setting pc to next_pc
					if end_fetch then
						pc <= next_pc;
					end if;
					if early_execute or not ready_to_execute then
						case opcode(7 downto 0) is
							--nops
							when x"1c" | x"3c" | x"5c" | x"7c" | x"dc" | x"fc" =>
								if subcycle(2 downto 0) = "010" then
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								else
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								end if;
							--indirect, x undocumented (5)
							when x"03" | x"23" | x"43" | x"63" | x"c3" | x"e3" =>
								case subcycle(2 downto 0) is
									when "001" =>
										op_byte_2 <= std_logic_vector(unsigned(din) + unsigned(x));
									when "010" =>
										addr_calc2(7 downto 0) <= din;
									when "011" =>
										addr_calc2(15 downto 8) <= din;
									when others =>
										null;
								end case;
							--indirect, x (5)
							when x"01" | x"21" | x"41" | x"61" | x"81" | x"83" | x"a1" | x"a3" | x"c1" | x"e1" =>
								case subcycle(2 downto 0) is
									when "001" =>
										null;
									when "010" =>
										op_byte_2 <= op_byte_2_plus_x;
									when "011" =>
										addr_calc2(7 downto 0) <= din;
									when others =>
										addr_calc2(15 downto 8) <= din;
								end case;
							--indirect, y (4)
							when x"11" | x"13" | x"31" | x"33" | x"51" | x"53" | x"71" | x"73" | x"91" | x"b1" | x"b3" | x"d1" | x"d3" | x"f1" | x"f3" =>
								case subcycle(2 downto 0) is
									when "001" =>
									when "010" =>
									when "011" =>
										addr_calc2 <= din & indirect_y_addr(7 downto 0);
										if indirect_y_addr(8) then
											extra_cycle <= "0001";
										else
											extra_cycle <= "0000";
										end if;
									when others =>
										addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								end case;
							--absolute, x (3)
							when	x"1d" | x"1e" | x"1f" | x"3d" | x"3e" | x"3f" | x"5d" | x"5f" | 
									x"7d" | x"7e" | x"7f" | x"bc" | x"bd" | x"dd" | x"de" | x"df" | x"fd" | x"ff" =>
								case subcycle(2 downto 0) is
									when "010" =>
										addr_calc2 <= din & absolute_x_addr(7 downto 0);
										if opcode(7 downto 0) = x"9d" then
											extra_cycle <= "0000";
										elsif absolute_x_addr(8) then
											extra_cycle <= "0001";
										else
											extra_cycle <= "0000";
										end if;
									when others =>
										addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
								end case;
							when x"9d" =>
								case subcycle(2 downto 0) is
									when "010" =>
										addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & x));
									when others => null;
								end case;
							--absolute, y (3)
							when x"19" | x"1b" | x"39" | x"3b" | x"59" | x"5b" | x"79" | x"7b" | x"97" | x"99" | x"b9" | x"be" | x"bf" | x"d9" | x"db" | x"f9" | x"fb" =>
								case subcycle(2 downto 0) is
									when "010" =>
										addr_calc2 <= din & absolute_y_addr(7 downto 0);
										if absolute_y_addr(8) then
											extra_cycle <= "0001";
										else
											extra_cycle <= "0000";
										end if;
									when others =>
										addr_calc2 <= std_logic_vector(unsigned(op_byte_3 & op_byte_2) + unsigned(x"00" & y));
								end case;
							--zero page, x (3)
							when	x"15" | x"16" | x"17" | x"35" | x"36" | x"37" | x"55" | x"57" | x"75" | x"76" | x"77" | 
									x"94" | x"95" | x"96" | x"b4" | x"b5" | x"b7" | x"d5" | x"d6" | x"d7" | x"f5" | x"f6" | x"f7" =>
								next_dout <= y;
							when others => null;
						end case;
					end if;
					if early_execute or ready_to_execute then
						case opcode(7 downto 0) is
							when	x"04" | x"0c" | x"14" | x"1a" | x"1c" | x"34" | x"3a" | x"3c" | x"44" | 
									x"54" | x"5a" | x"5c" | x"64" | x"74" | x"7a" | x"7c" | 
									x"80" | x"da" | x"d4" | x"dc" | x"ea" | x"f4" | x"fa" | x"fc" =>
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"4c" =>
								pc <= din & op_byte_2;
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"20" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										addr_calc2 <= pc;
										next_dout <= pc(15 downto 8);
									when "001" =>
										next_dout <= addr_calc2(7 downto 0);
										sp <= sp_minus;
									when "010" =>
										sp <= sp_minus;
									when others =>
										pc <= op_byte_3 & op_byte_2;
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							when x"24" | x"2c" =>
								flags(FLAG_OVERFLOW) <= din(FLAG_OVERFLOW);
								flags(FLAG_NEGATIVE) <= din(FLAG_NEGATIVE);
								if (din and a) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"18" | x"38" =>
								flags(FLAG_CARRY) <= opcode(5);
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"78" =>
								flags(FLAG_INTERRUPT) <= '1';
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"b8" =>
								flags(FLAG_OVERFLOW) <= '0';
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"d8" | x"f8" =>
								flags(FLAG_DECIMAL) <= opcode(5);
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"6c" =>
								case execution_cycle(1 downto 0) is
									when "00" =>
										addr_calc2(7 downto 0) <= din;
									when others => 
										pc <= din & addr_calc2(7 downto 0);
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							when x"01" | x"05" | x"09" | x"0d" | x"11" | x"15" | x"19" | x"1d" =>
								a <= a or din;
								if (a or din) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= a(7) or din(7);
								pc <= next_pc;
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"21" | x"25" | x"29" | x"2d" | x"31" | x"35" | x"39" | x"3d" =>
								a <= a and din;
								if (a and din) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= a(7) and din(7);
								pc <= next_pc;
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"41" | x"45" | x"49" | x"4d" | x"51" | x"55" | x"59" | x"5d" =>
								a <= a xor din;
								if (a xor din) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= a(7) xor din(7);
								pc <= next_pc;
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"c0" | x"c1" | x"c4" | x"c5" | x"c9" | x"cc" | x"cd" | x"d1" | x"d5" | x"d9" | x"dd" | x"e0" | x"e4" | x"ec" =>
								if sub = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								if sub_in >= din then
									flags(FLAG_CARRY) <= '1';
								else
									flags(FLAG_CARRY) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= sub(7);
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"e1" | x"e5" | x"e9" | x"eb" | x"ed" | x"f1" | x"f5" | x"f9" | x"fd" =>
								a <= sbc_result(7 downto 0);
								if sbc_result(7 downto 0) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= sbc_result(7);
								flags(FLAG_CARRY) <= not sbc_result(8);
								flags(FLAG_OVERFLOW) <= sbc_overflow;
								pc <= next_pc;
								instruction_toggle_pre <= not instruction_toggle_pre;
								extra_cycle <= "0000";
								subcycle <= (others => '0');
								opcode(8) <= '0';
							--dcp
							when x"c3" | x"c7" | x"cf" | x"d3" | x"d7" | x"db" | x"df" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										next_dout <= din;
									when "001" =>
										next_dout <= std_logic_vector(unsigned(next_dout) - 1);
									when others =>
										if subm = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										if sub_in >= next_dout then
											flags(FLAG_CARRY) <= '1';
										else
											flags(FLAG_CARRY) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= subm(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							--isb
							when x"e3" | x"e7" | x"ef" | x"f3" | x"f7" | x"fb" | x"ff" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										next_dout <= din;
									when "001" =>
										next_dout <= std_logic_vector(unsigned(next_dout) + 1);
									when others =>
										a <= sbcm_result(7 downto 0);
										if sbcm_result(7 downto 0) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= sbcm_result(7);
										flags(FLAG_CARRY) <= not sbcm_result(8);
										flags(FLAG_OVERFLOW) <= sbcm_overflow;
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							--slo
							when x"03" | x"07" | x"0f" | x"13" | x"17" | x"1b" | x"1f" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										next_dout <= din;
									when "001" =>
										next_dout <= next_dout(6 downto 0) & '0';
										flags(FLAG_CARRY) <= next_dout(7);
									when others =>
										a <= a or next_dout;
										if (a or next_dout) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) or next_dout(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							--rla
							when x"23" | x"27" | x"2f" | x"33" | x"37" | x"3b" | x"3f" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										next_dout <= din;
									when "001" =>
										next_dout <= next_dout(6 downto 0) & flags(FLAG_CARRY);
										flags(FLAG_CARRY) <= next_dout(7);
									when others =>
										a <= a and next_dout;
										if (a and next_dout) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) and next_dout(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							--sre
							when x"43" | x"47" | x"4f" | x"53" | x"57" | x"5b" | x"5f" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										next_dout <= din;
									when "001" =>
										next_dout <= '0' & next_dout(7 downto 1);
										flags(FLAG_CARRY) <= next_dout(0);
									when others =>
										a <= a xor next_dout;
										if (a xor next_dout) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) xor next_dout(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							--rra
							when x"63" | x"67" | x"6f" | x"73" | x"77" | x"7b" | x"7f" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										next_dout <= din;
									when "001" =>
										next_dout <= flags(FLAG_CARRY) & next_dout(7 downto 1);
										flags(FLAG_CARRY) <= next_dout(0);
									when others =>
										a <= adcm_result(7 downto 0);
										if adcm_result(7 downto 0) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= adcm_result(7);
										flags(FLAG_CARRY) <= adcm_result(8);
										flags(FLAG_OVERFLOW) <= adcm_overflow;
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							--lax
							when x"a3" | x"af" | x"b3" | x"bf" =>
								x <= din;
								a <= din;
								if din = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= din(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"e6" | x"ee" | x"f6" | x"fe" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										next_dout <= std_logic_vector(unsigned(din) + 1);
									when "001" =>
										if next_dout = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= next_dout(7);
									when others =>
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							when x"c6" | x"ce" | x"d6" | x"de" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										next_dout <= std_logic_vector(unsigned(din) - 1);
									when "001" =>
										if next_dout = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= next_dout(7);
									when others =>
										instruction_toggle_pre <= not instruction_toggle_pre;
										extra_cycle <= "0000";
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							when x"0a" =>
								flags(FLAG_CARRY) <= a(7);
								if a(6 downto 0) = "0000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= a(6);
								a <= a(6 downto 0) & '0';
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"06" | x"0e" | x"16" | x"1e" =>
								if not execution_cycle(0) then
									flags(FLAG_CARRY) <= din(7);
									if din(6 downto 0) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(6);
									next_dout <= din(6 downto 0) & '0';
								else
									extra_cycle <= "0000";
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
								end if;
							when x"4a" =>
								flags(FLAG_CARRY) <= a(0);
								if a(7 downto 1) = "0000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= '0';
								a <= '0' & a(7 downto 1);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"46" | x"4e" | x"56" | x"5e" =>
								if not execution_cycle(0) then
									flags(FLAG_CARRY) <= din(0);
									if din(7 downto 1) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= '0';
									next_dout <= '0' & din(7 downto 1);
								else
									extra_cycle <= "0000";
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
								end if;
							when x"2a" =>
								flags(FLAG_CARRY) <= a(7);
								if (a(6 downto 0) & flags(FLAG_CARRY)) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= a(6);
								a <= a(6 downto 0) & flags(FLAG_CARRY);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"26" | x"2e" | x"36" | x"3e" =>
								if not execution_cycle(0) then
									flags(FLAG_CARRY) <= din(7);
									if (din(6 downto 0) & flags(FLAG_CARRY)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(6);
									next_dout <= din(6 downto 0) & flags(FLAG_CARRY);
								else
									extra_cycle <= "0000";
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
								end if;
							when x"6a" =>
								flags(FLAG_CARRY) <= a(0);
								if (flags(FLAG_CARRY) & a(7 downto 1)) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= flags(FLAG_CARRY);
								a <= flags(FLAG_CARRY) & a(7 downto 1);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"66" | x"6e" | x"76" | x"7e" =>
								if not execution_cycle(0) then
									flags(FLAG_CARRY) <= din(0);
									if (flags(FLAG_CARRY) & din(7 downto 1)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= flags(FLAG_CARRY);
									next_dout <= flags(FLAG_CARRY) & din(7 downto 1);
								else
									extra_cycle <= "0000";
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
								end if;
							when x"61" | x"65" | x"69" | x"6d" | x"71" | x"75" | x"79" | x"7d" =>
								a <= adc_result(7 downto 0);
								if adc_result(7 downto 0) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= adc_result(7);
								flags(FLAG_CARRY) <= adc_result(8);
								flags(FLAG_OVERFLOW) <= adc_overflow;
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"88" | x"c8" =>
								y <= inc_out;
								flags(FLAG_NEGATIVE) <= inc_out(7);
								if inc_out = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"ca" | x"e8" =>
								x <= inc_out;
								flags(FLAG_NEGATIVE) <= inc_out(7);
								if inc_out = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"a1" | x"a5" | x"a9" | x"ad" | x"b1" | x"b5" | x"b9" | x"bd" =>
								a <= din;
								if din = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= din(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"a2" | x"a6" | x"ae" | x"b6" | x"be" =>
								x <= din;
								if din = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= din(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"a7" | x"b7" =>
								x <= din;
								a <= din;
								if din = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= din(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when	x"81" | x"83" | x"84" | x"85" | x"86" | x"87" | x"8c" | x"8d" | 
									x"8e" | x"8f" | x"91" | x"94" | x"95" | x"96" | x"97" | x"99" | x"9d" =>
								if not execution_cycle(0) then
										case opcode(1 downto 0) is
											when "00" => next_dout <= y;
											when "10" => next_dout <= x;
											when "01" => next_dout <= a;
											when others => next_dout <= a and x;
										end case;
								else
										extra_cycle <= "0000";
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end if;
							when x"a0" | x"a4" | x"ac" | x"b4" | x"bc" =>
								y <= din;
								if din = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= din(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"a8" =>
								y <= a;
								if a = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= a(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"98" =>
								a <= y;
								if y = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= y(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"8a" =>
								a <= x;
								if x = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= x(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"9a" =>
								sp <= x;
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"aa" =>
								x <= a;
								if a = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= a(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"ba" =>
								x <= sp;
								if sp = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= sp(7);
								extra_cycle <= "0000";
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							when x"40" =>
								case execution_cycle(2 downto 0) is
									when "000" => null;
									when "001" =>
										sp <= sp_plus;
									when "010" =>
										flags <= din;
										flags(FLAG_BREAK) <= '0';
										flags(FLAG_UNUSED) <= '1';
										sp <= sp_plus;
									when "011" =>
										pc(7 downto 0) <= din;
										sp <= sp_plus;
									when others =>
										pc(15 downto 8) <= din;
										extra_cycle <= "0000";
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							when x"60" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
										sp <= sp_plus;
									when "001" =>
										sp <= sp_plus;
										pc(7 downto 0) <= din;
									when "010" =>
										pc(15 downto 8) <= din;
									when "011" =>
										pc <= std_logic_vector(unsigned(pc) + 1);
									when others =>
										extra_cycle <= "0000";
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
							when x"08" | x"48" =>
								if not execution_cycle(0) then
									pc <= next_pc;
									if opcode(6) then
										next_dout <= a;
									else
										next_dout <= flags;
										next_dout(FLAG_BREAK) <= '1';
									end if;
								else
									sp <= sp_minus;
									extra_cycle <= "0000";
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
								end if;
							when x"28" | x"68" =>
								case execution_cycle(2 downto 0) is
									when "000" =>
									when "001" =>
										sp <= sp_plus;
									when others =>
										if opcode(6) then
											a <= din;
											if din = "00000000" then
												flags(FLAG_ZERO) <= '1';
											else
												flags(FLAG_ZERO) <= '0';
											end if;
											flags(FLAG_NEGATIVE) <= din(7);
										else
											flags <= din;
											flags(FLAG_BREAK) <= '0';
											flags(FLAG_UNUSED) <= '1';
										end if;
										extra_cycle <= "0000";
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
								end case;
								when x"10" | x"30" | x"50" | x"70" | x"90" | x"b0" | x"d0" | x"f0" =>
									case execution_cycle(2 downto 0) is
										when "000" =>
											pc <= next_pc;
											if flag_check then
												extra_cycle <= "0000";
												instruction_toggle_pre <= not instruction_toggle_pre;
												subcycle <= (others => '0');
												opcode(8) <= '0';
											else
												pc <= addr_calc1;
												addr_calc2 <= std_logic_vector(unsigned(pc(15 downto 0)) + unsigned(din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din));
											end if;
										when "001" =>
											if pc(15 downto 8) = addr_calc2(15 downto 8) then
												extra_cycle <= "0000";
												instruction_toggle_pre <= not instruction_toggle_pre;
												subcycle <= (others => '0');
												opcode(8) <= '0';
											end if;
										when others =>
											extra_cycle <= "0000";
											instruction_toggle_pre <= not instruction_toggle_pre;
											subcycle <= (others => '0');
											opcode(8) <= '0';
									end case;
							when others => null;
						end case;
					end if;
				end if;
			end if;
		end if;
	end process;

end Behavioral;

