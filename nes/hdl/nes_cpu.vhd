library IEEE;
use ieee.std_logic_1164.all;
use ieee.std_logic_misc.all;

use IEEE.NUMERIC_STD.ALL;

entity clock_divider is
	Port (
		pause_cpu: in std_logic;
		reset: in std_logic;
		clock: in std_logic;
		c1: out std_logic;
		c2: out std_logic;
		c3: out std_logic;
		c4: out std_logic);
end clock_divider;

architecture Behavioral of clock_divider is
	signal counter: std_logic_vector(2 downto 0);
	signal counter2: std_logic_vector(2 downto 0);
	signal clocko: std_logic;
	signal clocko2: std_logic;
begin
	c1 <= clocko;
	c2 <= clocko2;
	process (reset, clock)
	begin
		if reset='1' then
			counter <= "000";
			counter2 <= "000";
			c4 <= '0';
			clocko <= '0';
			clocko2 <= '1';
		elsif rising_edge(clock) then
			if pause_cpu = '0' then
				c3 <= clocko;
				counter <= std_logic_vector(unsigned(counter(2 downto 0)) + "001");
				if counter = "101" then
					counter <= "000";
					clocko <= not clocko;
					clocko2 <= not clocko2;
				end if;
				if counter = "010" then
					counter2 <= "000";
					c4 <= not c4;
				end if;
			end if;
		end if;
	end process;
end Behavioral;

library IEEE;
use ieee.std_logic_1164.all;
use ieee.std_logic_misc.all;

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
	signal sp: std_logic_vector(7 downto 0) := x"fd";
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
	
	signal rw_address: std_logic_vector(15 downto 0);
	signal read_cycle: std_logic; --indicates a read cycle when true
	signal stall: std_logic;
	signal stall_clocked: std_logic;
	
	signal instruction_length: std_logic_vector(3 downto 0);
	signal done_fetching: std_logic;
	signal pc_increment: std_logic;
	
	signal instruction_toggle_pre: std_logic;
	
	signal addr_calc1: std_logic_vector(15 downto 0);
	signal addr_calc2: std_logic_vector(15 downto 0);
	signal indirect_x_addr: std_logic_vector(8 downto 0);
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
		c2 => clockb,
		c3 => clockm,
		c4 => ppu_clock
		);

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
		indirect_x_addr <= std_logic_vector(unsigned('0' & op_byte_3) + unsigned('0' & x));
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
			if reset_active then
				read_cycle <= '1';
				case subcycle is
					when "0000" =>
						rw_address <= pc;
					when "0001" =>
						rw_address <= pc;
					when "0010" =>
						rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
					when "0011" =>
						rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"00ff");
					when "0100" =>
						rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"00fe");
					when "0101" =>
						rw_address <= x"FFFC";
					when "0110" =>
						rw_address <= x"FFFD";
					when others =>
					   rw_address <= "XXXXXXXXXXXXXXXX";
				end case;
			else
				if opcode(8) = '0' then
					rw_address <= pc;
					read_cycle <= '1';
				elsif done_fetching = '0' then
					rw_address <= pc;
					read_cycle <= '1';
				elsif done_fetching = '1' then
					rw_address <= pc;
					read_cycle <= '1';
					case opcode(7 downto 0) is
						when x"20" =>
							case subcycle(2 downto 0) is
								when "011" =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '0';
									dout <= pc(15 downto 8);
								when "100" =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '0';
									dout <= pc(7 downto 0);
								when others =>
									rw_address <= pc;
									read_cycle <= '1';
							end case;
						when x"40" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '1';
							end case;
						when x"60" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '1';
								when "011" =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '1';
								when others =>
									rw_address <= pc;
									read_cycle <= '1';
							end case;
						when x"08" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '0';
									dout <= flags;
									dout(FLAG_BREAK) <= '1';
							end case;
						when x"48" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '0';
									dout <= a;
							end case;
						when x"28" | x"68" =>
							rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
							read_cycle <= '1';
						--nop indirect x
						when x"14" | x"34" | x"54" | x"74" | x"d4" | x"f4" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & op_byte_2; --not technically correct for last cycle, doesn't matter for zero page
									read_cycle <= '1';
							end case;
						--indirect x
						when x"01" | x"21" | x"41" | x"61" | x"a1" | x"a3" | x"c1" | x"e1" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" | "011" =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '1';
								when "100" =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + 1);
									read_cycle <= '1';
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '1';
							end case;
						--indirect x store
						when x"81" | x"83" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" | "011" =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '1';
								when "100" =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + 1);
									read_cycle <= '1';
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--indirect x/y rmw
						when x"03" | x"13" | x"23" | x"33" | x"43" | x"53" | x"63" | x"73" | x"c3" | x"e3" | x"f3" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '1';
								when "011" =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + 1);
									read_cycle <= '1';
								when "100" | "101" =>
									rw_address <= addr_calc2;
									read_cycle <= '1';
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--dcp indirect y write
						when x"d3" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
								when "011" =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + 1);
									read_cycle <= '1';
								when "100" | "101" =>
									rw_address <= addr_calc2;
									read_cycle <= '1';
								when others =>
									rw_address <= addr_calc2;
									dout <= next_dout;
									read_cycle <= '0';
							end case;
						--indirect y write
						when x"91" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
								when "011" =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + 1);
									read_cycle <= '1';
								when "100" =>
									rw_address <= addr_calc2;
									read_cycle <= '1';
								when others =>
									rw_address <= addr_calc2;
									dout <= next_dout;
									read_cycle <= '0';
							end case;
						--indirect y read
						when x"11" | x"31" | x"51" | x"71" | x"b1" | x"b3" | x"d1" | x"f1" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
								when "011" =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + 1);
									read_cycle <= '1';
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '1';
							end case;
						--indirect jmp
						when x"6c" =>
							case subcycle(2 downto 0) is
								when "011" =>
									rw_address <= op_byte_3 & op_byte_2;
									read_cycle <= '1';
								when "100" =>
									rw_address <= op_byte_3 & std_logic_vector(unsigned(op_byte_2) + 1);
									read_cycle <= '1';
								when others =>
									rw_address <= pc;
									read_cycle <= '1';
							end case;
						--zero page y write
						when x"96" | x"97" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + unsigned(y));
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--zero page x write
						when x"94" | x"95" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--zero page y
						when x"b7" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + unsigned(y));
									read_cycle <= '1';
							end case;
						--zero page x
						when x"15" | x"35" | x"55" | x"75" | x"b4" | x"b5" | x"d5" | x"f5" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
									read_cycle <= '1';
							end case;
						--zero page y
						when x"b6" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + unsigned(y));
									read_cycle <= '1';
							end case;
						--zero page read
						when x"04" | x"24" | x"44" | x"64" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
							end case;
						--zero page read
						when x"05" | x"25" | x"45" | x"65" | x"a4" | x"a5" | x"a6" | x"a7" | x"c4" | x"c5" | x"e4" | x"e5" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '1';
							end case;
						--zero page x rmw
						when x"16" | x"17" | x"36" | x"37" | x"56" | x"57" | x"76" | x"77" | x"d6" | x"d7" | x"f6" | x"f7" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
								when "011" =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--zero page rmw
						when x"06" | x"07" | x"26" | x"27" | x"46" | x"47" | x"66" | x"67" | x"c6" | x"c7" | x"e6" | x"e7" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '1';
								when "011" =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '0';
									dout <= din;
								when "100" =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '0';
									dout <= next_dout;
								when others =>
									rw_address <= pc;
									read_cycle <= '1';
							end case;
						--zero page write
						when x"84" | x"85" | x"86" | x"87" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--absolute write a
						when x"8d" | x"8f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= op_byte_3 & op_byte_2;
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--absolute write x
						when x"8e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= op_byte_3 & op_byte_2;
									read_cycle <= '0';
									dout <= x;
							end case;
						--absolute write y
						when x"8c" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= op_byte_3 & op_byte_2;
									read_cycle <= '0';
									dout <= y;
							end case;
						--absolute rmw
						when x"0e" | x"0f" | x"2e" | x"2f" | x"4e" | x"4f" | x"6e" | x"6f" | x"ce" | x"cf" | x"ee" | x"ef" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "011" =>
									rw_address <= op_byte_3 & op_byte_2;
									read_cycle <= '1';
								when "100" =>
									read_cycle <= '0';
									rw_address <= op_byte_3 & op_byte_2;
									dout <= din;
								when others =>
									read_cycle <= '0';
									rw_address <= op_byte_3 & op_byte_2;
									dout <= next_dout;
							end case;
						--absolute read
						when	x"0c" | x"0d" | x"2c" | x"2d" | x"4d" | x"6d" | x"ac" | x"ad" | 
								x"ae" | x"af" | x"cc" | x"cd" | x"ec" | x"ed" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= op_byte_3 & op_byte_2;
									read_cycle <= '1';
							end case;
						--absolute y write
						when x"99" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "011" =>
									rw_address <= din & std_logic_vector(unsigned(op_byte_2) + unsigned(y));
									read_cycle <= '1';
								when others =>
									rw_address <=  std_logic_vector(unsigned(op_byte_3 & op_byte_2) + unsigned(x"00" & y));
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--absolute x/y rmw
						when	x"1b" | x"1e" | x"1f" | x"3b" | x"3e" | x"3f" | x"5b" | x"5e" | 
								x"5f" | x"7b" | x"7e" | x"7f" | x"db" | x"de" | x"df" | x"fb" | 
								x"fe" | x"ff" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "011" | "100" =>
									rw_address <= addr_calc2;
									read_cycle <= '1';
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--absolute x/y write
						when x"9d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "011" =>
									rw_address <= addr_calc2;
									read_cycle <= '1';
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '0';
									dout <= next_dout;
							end case;
						--absolute x/y read
						when	x"19" | x"1c" | x"1d" | x"39" | x"3c" | x"3d" | x"59" | x"5c" | 
								x"5d" | x"79" | x"7c" | x"7d" | x"b9" | x"bc" | x"bd" | x"be" | 
								x"bf" | x"d9" | x"dc" | x"dd" | x"f9" | x"fc" | x"fd" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= addr_calc2;
									read_cycle <= '1';
							end case;
						when others =>
							rw_address <= pc;
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
					x"c9" | x"d0" | x"d1" | x"d4" | x"d5" | x"d6" | x"e0" | x"d3" | 
					x"d7" | x"e1" | x"e3" | x"e4" | x"e5" | x"e6" | x"e7" | x"e9" | 
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
		addr_calc1 <= std_logic_vector(unsigned(next_pc(15 downto 0)) + 
			unsigned(din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din));
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
			sp <= x"FD";
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
					when "000" =>
					when "001" =>
					when "010" =>
					when "011" =>
					when "100" =>
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
					case opcode(7 downto 0) is
						when x"1a" | x"3a" | x"5a" | x"7a" | x"da" | x"ea" | x"fa" =>
							if subcycle(0) then
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							end if;
						when x"1c" | x"3c" | x"5c" | x"7c" | x"dc" | x"fc" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"80" =>
							pc <= next_pc;
							if subcycle(0) then
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							end if;
						when x"04" | x"44" | x"64" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"14" | x"34" | x"54" | x"74" | x"d4" | x"f4" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"0c" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"20" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) - "00000001");
								when "100" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) - "00000001");
								when others =>
									pc <= op_byte_3 & op_byte_2;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"24" =>
							if subcycle(1) then
								flags(FLAG_OVERFLOW) <= din(FLAG_OVERFLOW);
								flags(FLAG_NEGATIVE) <= din(FLAG_NEGATIVE);
								if (din and a) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							else
								pc <= next_pc;
							end if;
						when x"2c" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									flags(FLAG_OVERFLOW) <= din(FLAG_OVERFLOW);
									flags(FLAG_NEGATIVE) <= din(FLAG_NEGATIVE);
									if (din and a) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--flag modifying instructions
						when x"18" =>
							flags(FLAG_CARRY) <= '0';
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"38" =>
							flags(FLAG_CARRY) <= '1';
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"78" =>
							flags(FLAG_INTERRUPT) <= '1';
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"b8" =>
							flags(FLAG_OVERFLOW) <= '0';
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"d8" =>
							flags(FLAG_DECIMAL) <= '0';
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"f8" =>
							flags(FLAG_DECIMAL) <= '1';
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"4c" =>
							pc <= din & op_byte_2;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"6c" =>
							case subcycle(2 downto 0) is
								when "010" =>
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when others =>
									pc <= din & addr_calc2(7 downto 0);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"86" =>
							if subcycle(1) then
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							else
								pc <= next_pc;
								next_dout <= x;
							end if;
						when x"85" =>
							if subcycle(1) then
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							else
								pc <= next_pc;
								next_dout <= a;
							end if;
						when x"87" =>
							if subcycle(1) then
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							else
								pc <= next_pc;
								next_dout <= a and x;
							end if;
						when x"94" | x"95" | x"96" | x"97"=>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									case opcode(7 downto 0) is
										when x"94" => next_dout <= y;
										when x"95" => next_dout <= a;
										when x"97" => next_dout <= a and x;
										when others => next_dout <= x;
									end case;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--logic instructions
						when x"09" =>
							a <= a or din;
							if (a or din) = "00000000" then
								flags(FLAG_ZERO) <= '1';
							else
								flags(FLAG_ZERO) <= '0';
							end if;
							flags(FLAG_NEGATIVE) <= a(7) or din(7);
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"05" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									a <= a or din;
									if (a or din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) or din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"15" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= y;
								when others =>
									a <= a or din;
									if (a or din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) or din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"0d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									a <= a or din;
									if (a or din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) or din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"1d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
										a <= a or din;
										if (a or din) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) or din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= a or din;
									if (a or din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) or din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"19" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_y_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & y));
									if absolute_y_addr(8) = '0' then
										a <= a or din;
										if (a or din) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) or din(7);
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= a or din;
									if (a or din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) or din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"01" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
								when others =>
									a <= a or din;
									if (a or din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) or din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"11" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									if flags(FLAG_ZERO) = '0' then
										a <= a or din;
										if (a or din) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) or din(7);
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when others =>
									a <= a or din;
									if (a or din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) or din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"29" =>
							a <= a and din;
							if (a and din) = "00000000" then
								flags(FLAG_ZERO) <= '1';
							else
								flags(FLAG_ZERO) <= '0';
							end if;
							flags(FLAG_NEGATIVE) <= a(7) and din(7);
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"25" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									a <= a and din;
									if (a and din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) and din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"35" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= y;
								when others =>
									a <= a and din;
									if (a and din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) and din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"2d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									a <= a and din;
									if (a and din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) and din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"3d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
										a <= a and din;
										if (a and din) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) and din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= a and din;
									if (a and din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) and din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"39" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_y_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & y));
									if absolute_y_addr(8) = '0' then
										a <= a and din;
										if (a and din) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) and din(7);
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= a and din;
									if (a and din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) and din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"21" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
								when others =>
									a <= a and din;
									if (a and din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) and din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"31" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									if flags(FLAG_ZERO) = '0' then
										a <= a and din;
										if (a and din) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) and din(7);
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when others =>
									a <= a and din;
									if (a and din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) and din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"49" =>
							a <= a xor din;
							if (a xor din) = "00000000" then
								flags(FLAG_ZERO) <= '1';
							else
								flags(FLAG_ZERO) <= '0';
							end if;
							flags(FLAG_NEGATIVE) <= a(7) xor din(7);
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"45" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									a <= a xor din;
									if (a xor din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) xor din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"55" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= y;
								when others =>
									a <= a xor din;
									if (a xor din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) xor din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"4d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									a <= a xor din;
									if (a xor din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) xor din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"5d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
										a <= a xor din;
										if (a xor din) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) xor din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= a xor din;
									if (a xor din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) xor din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"59" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_y_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & y));
									if absolute_y_addr(8) = '0' then
										a <= a xor din;
										if (a xor din) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) xor din(7);
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= a xor din;
									if (a xor din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) xor din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"41" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
								when others =>
									a <= a xor din;
									if (a xor din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) xor din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"51" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									if flags(FLAG_ZERO) = '0' then
										a <= a xor din;
										if (a xor din) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= a(7) xor din(7);
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when others =>
									a <= a xor din;
									if (a xor din) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= a(7) xor din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"c0" | x"c9" | x"e0" =>
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
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"c4" | x"c5" | x"e4" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"d5" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when others =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"cc" | x"cd" | x"ec"=>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"dd" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
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
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"d9" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_y_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & y));
									if absolute_y_addr(8) = '0' then
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
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"c1" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
								when others =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--dcp indirect x
						when x"c3" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
									op_byte_2 <= std_logic_vector(unsigned(din) + unsigned(x));
								when "010" =>
									addr_calc2(7 downto 0) <= din;
								when "011" =>
									addr_calc2(15 downto 8) <= din;
								when "100" =>
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--dcp zero page
						when x"c7" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= din;
								when "011" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--dcp absolute
						when x"cf" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= din;
								when "100" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--dcp indirect y
						when x"d3" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--dcp zero page x
						when x"d7" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--dcp absolute y
						when x"db" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & y));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "101" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--dcp absolute x
						when x"df" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & x));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_x_addr(8);
								when "101" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--isb indirect x
						when x"e3" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
									op_byte_2 <= std_logic_vector(unsigned(din) + unsigned(x));
								when "010" =>
									addr_calc2(7 downto 0) <= din;
								when "011" =>
									addr_calc2(15 downto 8) <= din;
								when "100" =>
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--isb zero page
						when x"e7" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= din;
								when "011" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--isb absolute
						when x"ef" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= din;
								when "100" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--isb indirect y
						when x"f3" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--isb zero page x
						when x"f7" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--isb absolute y
						when x"fb" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & y));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "101" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--isb absolute x
						when x"ff" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & x));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_x_addr(8);
								when "101" =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--slo indirect x
						when x"03" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
									op_byte_2 <= std_logic_vector(unsigned(din) + unsigned(x));
								when "010" =>
									addr_calc2(7 downto 0) <= din;
								when "011" =>
									addr_calc2(15 downto 8) <= din;
								when "100" =>
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--slo zero page
						when x"07" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= din;
								when "011" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--slo absolute
						when x"0f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= din;
								when "100" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--slo indirect y
						when x"13" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--slo zero page x
						when x"17" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--slo absolute y
						when x"1b" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & y));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "101" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--slo absolute x
						when x"1f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & x));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_x_addr(8);
								when "101" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rla indirect x
						when x"23" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
									op_byte_2 <= std_logic_vector(unsigned(din) + unsigned(x));
								when "010" =>
									addr_calc2(7 downto 0) <= din;
								when "011" =>
									addr_calc2(15 downto 8) <= din;
								when "100" =>
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rla zero page
						when x"27" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= din;
								when "011" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rla absolute
						when x"2f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= din;
								when "100" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rla indirect y
						when x"33" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rla zero page x
						when x"37" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rla absolute y
						when x"3b" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & y));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "101" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rla absolute x
						when x"3f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & x));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_x_addr(8);
								when "101" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--sre indirect x
						when x"43" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
									op_byte_2 <= std_logic_vector(unsigned(din) + unsigned(x));
								when "010" =>
									addr_calc2(7 downto 0) <= din;
								when "011" =>
									addr_calc2(15 downto 8) <= din;
								when "100" =>
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--sre zero page
						when x"47" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= din;
								when "011" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--sre absolute
						when x"4f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= din;
								when "100" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--sre indirect y
						when x"53" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--sre zero page x
						when x"57" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--sre absolute y
						when x"5b" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & y));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "101" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--sre absolute x
						when x"5f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & x));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_x_addr(8);
								when "101" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rra indirect x
						when x"63" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
									op_byte_2 <= std_logic_vector(unsigned(din) + unsigned(x));
								when "010" =>
									addr_calc2(7 downto 0) <= din;
								when "011" =>
									addr_calc2(15 downto 8) <= din;
								when "100" =>
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rra zero page
						when x"67" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= din;
								when "011" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rra absolute
						when x"6f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= din;
								when "100" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rra indirect y
						when x"73" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when "101" =>
									next_dout <= din;
								when "110" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rra zero page x
						when x"77" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din;
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rra absolute y
						when x"7b" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & y));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "101" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--rra absolute x
						when x"7f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & x));
								when "011" =>
								when "100" =>
									next_dout <= din;
									flags(FLAG_ZERO) <= indirect_x_addr(8);
								when "101" =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"d1" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									if flags(FLAG_ZERO) = '0' then
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
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when others =>
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
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--math instructions
						when x"e6" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= std_logic_vector(unsigned(din) + 1);
								when "011" =>
									if next_dout = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(7);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"f6" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= std_logic_vector(unsigned(din) + 1);
								when "100" =>
									if next_dout = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(7);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"ee" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= std_logic_vector(unsigned(din) + 1);
								when "100" =>
									if next_dout = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(7);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"fe" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(op_byte_3 & op_byte_2) + unsigned(x"00" & x));
									next_dout <= din;
								when "100" =>
								when "101" =>
									next_dout <= std_logic_vector(unsigned(next_dout) + 1);
								when others =>
									if next_dout = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"c6" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= std_logic_vector(unsigned(din) - 1);
								when "011" =>
									if next_dout = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(7);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"d6" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= std_logic_vector(unsigned(din) - 1);
								when "100" =>
									if next_dout = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(7);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"ce" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= std_logic_vector(unsigned(din) - 1);
								when "100" =>
									if next_dout = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(7);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"de" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(op_byte_3 & op_byte_2) + unsigned(x"00" & x));
									next_dout <= din;
								when "100" =>
								when "101" =>
									next_dout <= std_logic_vector(unsigned(next_dout) - 1);
								when others =>
									if next_dout = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"0a" =>
							if subcycle(0) then
								flags(FLAG_CARRY) <= a(7);
								if a(6 downto 0) = "0000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= a(6);
								a <= a(6 downto 0) & '0';
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							end if;
						when x"06" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= din(6 downto 0) & '0';
									flags(FLAG_CARRY) <= din(7);
									if din(6 downto 0) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(6);
								when "011" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"16" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din(6 downto 0) & '0';
									flags(FLAG_CARRY) <= din(7);
									if din(6 downto 0) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(6);
								when "100" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"0e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= din(6 downto 0) & '0';
									flags(FLAG_CARRY) <= din(7);
									if din(6 downto 0) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(6);
								when "100" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"1e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(op_byte_3 & op_byte_2) + unsigned(x"00" & x));
									next_dout <= din;
								when "100" =>
								when "101" =>
									next_dout <= next_dout(6 downto 0) & '0';
									flags(FLAG_CARRY) <= next_dout(7);
									if next_dout(6 downto 0) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(6);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"4a" =>
							if subcycle(0) then
								flags(FLAG_CARRY) <= a(0);
								if a(7 downto 1) = "0000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= '0';
								a <= '0' & a(7 downto 1);
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							end if;
						when x"46" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= '0' & din(7 downto 1);
									flags(FLAG_CARRY) <= din(0);
									if din(7 downto 1) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= '0';
								when "011" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"56" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= '0' & din(7 downto 1);
									flags(FLAG_CARRY) <= din(0);
									if din(7 downto 1) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= '0';
								when "100" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"4e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= '0' & din(7 downto 1);
									flags(FLAG_CARRY) <= din(0);
									if din(7 downto 1) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= '0';
								when "100" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"5e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(op_byte_3 & op_byte_2) + unsigned(x"00" & x));
									next_dout <= din;
								when "100" =>
								when "101" =>
									next_dout <= '0' & next_dout(7 downto 1);
									flags(FLAG_CARRY) <= next_dout(0);
									if next_dout(7 downto 1) = "0000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= '0';
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"2a" =>
							if subcycle(0) then
								flags(FLAG_CARRY) <= a(7);
								if (a(6 downto 0) & flags(FLAG_CARRY)) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= a(6);
								a <= a(6 downto 0) & flags(FLAG_CARRY);
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							end if;
						when x"26" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= din(6 downto 0) & flags(FLAG_CARRY);
									flags(FLAG_CARRY) <= din(7);
									if (din(6 downto 0) & flags(FLAG_CARRY)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(6);
								when "011" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"36" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= din(6 downto 0) & flags(FLAG_CARRY);
									flags(FLAG_CARRY) <= din(7);
									if (din(6 downto 0) & flags(FLAG_CARRY)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(6);
								when "100" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"2e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= din(6 downto 0) & flags(FLAG_CARRY);
									flags(FLAG_CARRY) <= din(7);
									if (din(6 downto 0) & flags(FLAG_CARRY)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(6);
								when "100" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"3e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(op_byte_3 & op_byte_2) + unsigned(x"00" & x));
									next_dout <= din;
								when "100" =>
								when "101" =>
									next_dout <= next_dout(6 downto 0) & flags(FLAG_CARRY);
									flags(FLAG_CARRY) <= next_dout(7);
									if (next_dout(6 downto 0) & flags(FLAG_CARRY)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= next_dout(6);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"6a" =>
							if subcycle(0) then
								flags(FLAG_CARRY) <= a(0);
								if (flags(FLAG_CARRY) & a(7 downto 1)) = "00000000" then
									flags(FLAG_ZERO) <= '1';
								else
									flags(FLAG_ZERO) <= '0';
								end if;
								flags(FLAG_NEGATIVE) <= flags(FLAG_CARRY);
								a <= flags(FLAG_CARRY) & a(7 downto 1);
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							end if;
						when x"66" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= flags(FLAG_CARRY) & din(7 downto 1);
									flags(FLAG_CARRY) <= din(0);
									if (flags(FLAG_CARRY) & din(7 downto 1)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= flags(FLAG_CARRY);
								when "011" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"76" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									next_dout <= flags(FLAG_CARRY) & din(7 downto 1);
									flags(FLAG_CARRY) <= din(0);
									if (flags(FLAG_CARRY) & din(7 downto 1)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= flags(FLAG_CARRY);
								when "100" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"6e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= flags(FLAG_CARRY) & din(7 downto 1);
									flags(FLAG_CARRY) <= din(0);
									if (flags(FLAG_CARRY) & din(7 downto 1)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= flags(FLAG_CARRY);
								when "100" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"7e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(op_byte_3 & op_byte_2) + unsigned(x"00" & x));
									next_dout <= din;
								when "100" =>
								when "101" =>
									next_dout <= flags(FLAG_CARRY) & next_dout(7 downto 1);
									flags(FLAG_CARRY) <= next_dout(0);
									if (flags(FLAG_CARRY) & next_dout(7 downto 1)) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= flags(FLAG_CARRY);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"69" =>
							a <= adc_result(7 downto 0);
							if adc_result(7 downto 0) = "00000000" then
								flags(FLAG_ZERO) <= '1';
							else
								flags(FLAG_ZERO) <= '0';
							end if;
							flags(FLAG_NEGATIVE) <= adc_result(7);
							flags(FLAG_CARRY) <= adc_result(8);
							flags(FLAG_OVERFLOW) <= adc_overflow;
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"65" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									a <= adc_result(7 downto 0);
									if adc_result(7 downto 0) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= adc_result(7);
									flags(FLAG_CARRY) <= adc_result(8);
									flags(FLAG_OVERFLOW) <= adc_overflow;
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"75" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									next_dout <= y;
								when others =>
									a <= adc_result(7 downto 0);
									if adc_result(7 downto 0) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= adc_result(7);
									flags(FLAG_CARRY) <= adc_result(8);
									flags(FLAG_OVERFLOW) <= adc_overflow;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"6d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									a <= adc_result(7 downto 0);
									if adc_result(7 downto 0) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= adc_result(7);
									flags(FLAG_CARRY) <= adc_result(8);
									flags(FLAG_OVERFLOW) <= adc_overflow;
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"7d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
										a <= adc_result(7 downto 0);
										if adc_result(7 downto 0) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= adc_result(7);
										flags(FLAG_CARRY) <= adc_result(8);
										flags(FLAG_OVERFLOW) <= adc_overflow;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= adc_result(7 downto 0);
									if adc_result(7 downto 0) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= adc_result(7);
									flags(FLAG_CARRY) <= adc_result(8);
									flags(FLAG_OVERFLOW) <= adc_overflow;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"79" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_y_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & y));
									if absolute_y_addr(8) = '0' then
										a <= adc_result(7 downto 0);
										if adc_result(7 downto 0) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= adc_result(7);
										flags(FLAG_CARRY) <= adc_result(8);
										flags(FLAG_OVERFLOW) <= adc_overflow;
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= adc_result(7 downto 0);
									if adc_result(7 downto 0) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= adc_result(7);
									flags(FLAG_CARRY) <= adc_result(8);
									flags(FLAG_OVERFLOW) <= adc_overflow;
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"61" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
								when others =>
									a <= adc_result(7 downto 0);
									if adc_result(7 downto 0) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= adc_result(7);
									flags(FLAG_CARRY) <= adc_result(8);
									flags(FLAG_OVERFLOW) <= adc_overflow;
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"71" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									if flags(FLAG_ZERO) = '0' then
										a <= adc_result(7 downto 0);
										if adc_result(7 downto 0) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= adc_result(7);
										flags(FLAG_CARRY) <= adc_result(8);
										flags(FLAG_OVERFLOW) <= adc_overflow;
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when others =>
									a <= adc_result(7 downto 0);
									if adc_result(7 downto 0) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= adc_result(7);
									flags(FLAG_CARRY) <= adc_result(8);
									flags(FLAG_OVERFLOW) <= adc_overflow;
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"e9" | x"eb" =>
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
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"e5" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"f5" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when others =>
									a <= sbc_result(7 downto 0);
									if sbc_result(7 downto 0) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= sbc_result(7);
									flags(FLAG_CARRY) <= not sbc_result(8);
									flags(FLAG_OVERFLOW) <= sbc_overflow;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"ed" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"fd" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
										a <= sbc_result(7 downto 0);
										if sbc_result(7 downto 0) = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= sbc_result(7);
										flags(FLAG_CARRY) <= not sbc_result(8);
										flags(FLAG_OVERFLOW) <= sbc_overflow;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= sbc_result(7 downto 0);
									if sbc_result(7 downto 0) = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= sbc_result(7);
									flags(FLAG_CARRY) <= not sbc_result(8);
									flags(FLAG_OVERFLOW) <= sbc_overflow;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"f9" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_y_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & y));
									if absolute_y_addr(8) = '0' then
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
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"e1" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
								when others =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"f1" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									if flags(FLAG_ZERO) = '0' then
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
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when others =>
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
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"88" | x"c8" =>
							y <= inc_out;
							flags(FLAG_NEGATIVE) <= inc_out(7);
							if inc_out = "00000000" then
								flags(FLAG_ZERO) <= '1';
							else
								flags(FLAG_ZERO) <= '0';
							end if;
							pc <= next_pc;
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
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						--load instructions
						when x"a9" =>
							a <= din;
							if din = "00000000" then
								flags(FLAG_ZERO) <= '1';
							else
								flags(FLAG_ZERO) <= '0';
							end if;
							flags(FLAG_NEGATIVE) <= din(7);
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"a5" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"a7" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									x <= din;
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"b5" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when others =>
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"b7" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when others =>
									x <= din;
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"99" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when "011" =>
									next_dout <= a;
								when others =>
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"81" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
									next_dout <= a;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"83" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
									next_dout <= a and x;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"91" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
								when "100" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
									next_dout <= a;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"b9" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_y_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & y));
									if absolute_y_addr(8) = '0' then
										a <= din;
										if din = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"bf" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_y_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & y));
									if absolute_y_addr(8) = '0' then
										x <= din;
										a <= din;
										if din = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									x <= din;
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"a1" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
								when others =>
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"a3" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									op_byte_2 <= std_logic_vector(unsigned(op_byte_2) + unsigned(x));
								when "011" =>
									addr_calc2(7 downto 0) <= din;
								when "100" =>
									addr_calc2(15 downto 8) <= din;
								when others =>
									x <= din;
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"b1" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									if flags(FLAG_ZERO) = '0' then
										a <= din;
										if din = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when others =>
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"b3" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when "011" =>
									addr_calc2 <= din & indirect_y_addr(7 downto 0);
									flags(FLAG_ZERO) <= indirect_y_addr(8);
								when "100" =>
									if flags(FLAG_ZERO) = '0' then
										x <= din;
										a <= din;
										if din = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_3) + unsigned(x"00" & y));
								when others =>
									x <= din;
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"ad" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"af" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									x <= din;
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"bd" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
										a <= din;
										if din = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"a0" =>
							y <= din;
							if din = "00000000" then
								flags(FLAG_ZERO) <= '1';
							else
								flags(FLAG_ZERO) <= '0';
							end if;
							flags(FLAG_NEGATIVE) <= din(7);
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"a4" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									y <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"b6" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when others =>
									x <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"b4" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
								when others =>
									y <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"ac" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									y <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"bc" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_x_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & x));
									if absolute_x_addr(8) = '0' then
										y <= din;
										if din = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									y <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"a2" =>
							x <= din;
							if din = "00000000" then
								flags(FLAG_ZERO) <= '1';
							else
								flags(FLAG_ZERO) <= '0';
							end if;
							flags(FLAG_NEGATIVE) <= din(7);
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"a6" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									x <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"ae" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									x <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"be" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= din & absolute_y_addr(7 downto 0);
								when "011" =>
									addr_calc2 <= std_logic_vector(unsigned(addr_calc2(15 downto 8) & op_byte_2) + unsigned(x"00" & y));
									if absolute_y_addr(8) = '0' then
										x <= din;
										if din = "00000000" then
											flags(FLAG_ZERO) <= '1';
										else
											flags(FLAG_ZERO) <= '0';
										end if;
										flags(FLAG_NEGATIVE) <= din(7);
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									x <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"98" =>
							a <= y;
							if y = "00000000" then
								flags(FLAG_ZERO) <= '1';
							else
								flags(FLAG_ZERO) <= '0';
							end if;
							flags(FLAG_NEGATIVE) <= y(7);
							pc <= next_pc;
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
							pc <= next_pc;
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
							pc <= next_pc;
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
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						when x"9a" =>
							sp <= x;
							pc <= next_pc;
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
							pc <= next_pc;
							instruction_toggle_pre <= not instruction_toggle_pre;
							subcycle <= (others => '0');
							opcode(8) <= '0';
						--store instructions
						when x"8d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									next_dout <= a;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"8e" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									next_dout <= x;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"8f" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									next_dout <= a and x;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"9d" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
									addr_calc2 <= std_logic_vector(unsigned(din & op_byte_2) + unsigned(x"00" & x));
									next_dout <= a;
								when "011" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"84" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
									next_dout <= y;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"8c" =>
							case subcycle(2 downto 0) is
								when "010" =>
									pc <= next_pc;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--stack instructions
						when x"40" =>
							case subcycle(2 downto 0) is
								when "001" =>
								when "010" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
								when "011" =>
									flags <= din;
									flags(FLAG_BREAK) <= '0';
									flags(FLAG_UNUSED) <= '1';
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
								when "100" =>
									pc <= pc(15 downto 8) & din;
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
								when others =>
									pc <= din & pc(7 downto 0);
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"60" =>
							case subcycle(2 downto 0) is
								when "001" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
								when "010" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
									pc <= pc(15 downto 8) & din;
								when "011" =>
									pc <= din & pc(7 downto 0);
								when "100" =>
									pc <= std_logic_vector(unsigned(pc) + 1);
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"08" | x"48" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when others =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) - "00000001");
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"28" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
								when others =>
									flags <= din;
									flags(FLAG_BREAK) <= '0';
									flags(FLAG_UNUSED) <= '1';
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"68" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
								when "010" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
								when others =>
									a <= din;
									if din = "00000000" then
										flags(FLAG_ZERO) <= '1';
									else
										flags(FLAG_ZERO) <= '0';
									end if;
									flags(FLAG_NEGATIVE) <= din(7);
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						--branch instructions
						when x"10" | x"30" | x"50" | x"70" | x"90" | x"b0" | x"d0" | x"f0" =>
							case subcycle(2 downto 0) is
								when "001" =>
									pc <= next_pc;
									if flag_check then
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									else
										pc <= addr_calc1;
										addr_calc2 <= std_logic_vector(unsigned(pc(15 downto 0)) + unsigned(din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din(7) & din));
									end if;
								when "010" =>
									if pc(15 downto 8) = addr_calc2(15 downto 8) then
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									end if;
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when others =>
					end case;
				end if;
			end if;
		end if;
	end process;

end Behavioral;

