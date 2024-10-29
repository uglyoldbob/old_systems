library IEEE;
use ieee.std_logic_1164.all;
use ieee.std_logic_misc.all;

use IEEE.NUMERIC_STD.ALL;

entity clock_divider is
	Port (
		reset: in std_logic;
		clock: in std_logic;
		c1: out std_logic;
		c2: out std_logic;
		c3: out std_logic);
end clock_divider;

architecture Behavioral of clock_divider is
	signal counter: std_logic_vector(2 downto 0);
	signal clocko: std_logic;
	signal clocko2: std_logic;
begin
	c1 <= clocko;
	c2 <= clocko2;
	process (reset, clock)
	begin
		if reset='1' then
			counter <= "000";
			clocko <= '0';
			clocko2 <= '1';
		elsif rising_edge(clock) then
			c3 <= clocko;
			counter <= std_logic_vector(unsigned(counter(2 downto 0)) + "001");
			if counter = "110" then
				counter <= "000";
				clocko <= not clocko;
				clocko2 <= not clocko2;
			end if;
		end if;
	end process;
end Behavioral;

library IEEE;
use ieee.std_logic_1164.all;
use ieee.std_logic_misc.all;

use IEEE.NUMERIC_STD.ALL;

entity nes_cpu is
   Port (clock : in STD_LOGIC;
         audio : out STD_LOGIC_VECTOR (1 downto 0);
         address : out STD_LOGIC_VECTOR (15 downto 0);
			memory_start: out std_logic;
			memory_clock: out std_logic;
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
	signal clocka: std_logic;
	signal clockb: std_logic;
	signal clockm: std_logic;

	signal pc: std_logic_vector(15 downto 0);
	signal next_pc: std_logic_vector(15 downto 0);
	
	signal a: std_logic_vector(7 downto 0) := x"00";
	signal x: std_logic_vector(7 downto 0) := x"00";
	signal y: std_logic_vector(7 downto 0) := x"00";
	signal sp: std_logic_vector(7 downto 0) := x"fd";
	signal flags: std_logic_vector(7 downto 0) := x"24";
	
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
	signal instruction_toggle: std_logic;
	signal cycle_toggle: std_logic;
	
	signal cycle_counter: std_logic_vector(14 downto 0);
	
	signal addr_calc1: std_logic_vector(15 downto 0);
	signal addr_calc2: std_logic_vector(15 downto 0);
	
	signal sub_in: std_logic_vector(7 downto 0);
	signal sub: std_logic_vector(7 downto 0);

	signal adc_carry_in: std_logic_vector(8 downto 0);
	signal adc_result: std_logic_vector(8 downto 0);
	signal adc_overflow: std_logic;
	
	signal sbc_carry_in: std_logic_vector(8 downto 0);
	signal sbc_result: std_logic_vector(8 downto 0);
	signal sbc_overflow: std_logic;
	
	signal inc_out: std_logic_vector(7 downto 0);
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
		reset => reset,
		clock => clock,
		c1 => clocka,
		c2 => clockb,
		c3 => clockm
		);

	memory_clock <= clockm;
		
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
		
		sbc_carry_in <= (0 => not flags(FLAG_CARRY), others=>'0');
		sbc_result <= std_logic_vector(unsigned('0' & a) - unsigned('0' & din) - unsigned(sbc_carry_in));
		sbc_overflow <= (a(7) xor sbc_result(7)) and (din(7) xor sbc_result(7) xor '1');
	end process;
	
	process (all)
	begin
		case opcode(7 downto 0) is
			when x"c0" => sub_in <= y;
			when x"e0" => sub_in <= x;
			when others => sub_in <= a;
		end case;
		sub <= std_logic_vector(unsigned(sub_in) - unsigned(din));
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
	
	process (reset, clocka)
	begin
		if reset='1' then
			read_cycle <= '1';
			cycle_toggle <= '0';
			memory_start <= '0';
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
									read_cycle <= '1';
								when "100" =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '0';
									dout <= next_pc(15 downto 8);
								when "101" =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '0';
									dout <= next_pc(7 downto 0);
								when others =>
									rw_address <= pc;
									read_cycle <= '1';
							end case;
						when x"60" =>
							report("RTS MEMORY phase " & to_hstring(subcycle));
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when "010" =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '1';
								when "011" =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '1';
								when "100" =>
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
									report("Writing " & to_hstring(a) & " to " & to_hstring(std_logic_vector(unsigned(sp(7 downto 0)) + x"0100")));
									dout <= a;
							end case;
						when x"28" | x"68" =>
							case subcycle(2 downto 0) is
								when "001" | "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= std_logic_vector(unsigned(sp(7 downto 0)) + x"0100");
									read_cycle <= '1';
							end case;
						when x"24" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & din;
									read_cycle <= '1';
							end case;
						when x"85" =>
							case subcycle(2 downto 0) is
								when "001" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & op_byte_2;
									read_cycle <= '0';
									dout <= a;
							end case;
						when x"86" =>
							case subcycle(2 downto 0) is
								when "010" =>
									rw_address <= pc;
									read_cycle <= '1';
								when others =>
									rw_address <= x"00" & din;
									read_cycle <= '0';
									dout <= x;
							end case;
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
						when x"ad" | x"ae" =>
							rw_address <= op_byte_3 & op_byte_2;
							read_cycle <= '1';
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
			when x"09" | x"10" | x"24" | x"29" | x"30" | x"49" | x"50" | x"69" | 
				  x"70" | x"85" | x"86" | x"90" | x"a0" | x"a2" | x"a9" | x"b0" | 
				  x"c0" | x"c9" | x"d0" | x"e0" | x"e9" | x"f0" => instruction_length <= "0001";
			when x"20" | x"4c" | x"8e" | x"ad" | x"ae" => instruction_length <= "0010";
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
						when x"ea" =>
							if subcycle(0) then
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							end if;
						when x"20" =>
							case subcycle(2 downto 0) is
								when "001" =>
								when "010" =>
									pc <= next_pc;
								when "011" | "100" =>
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
						when x"85" | x"86" =>
							if subcycle(1) then
								report("Running command for 0x86");
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							else
								pc <= next_pc;
							end if;
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
						--math instructions
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
						when x"e9" =>
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
						when x"a9" | x"ad" =>
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
						when x"a2" | x"ae" =>
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
						when x"8e" =>
							if subcycle(2 downto 0) = "011" then
								instruction_toggle_pre <= not instruction_toggle_pre;
								subcycle <= (others => '0');
								opcode(8) <= '0';
							else
								pc <= next_pc;
							end if;
						--stack instructions
						when x"60" =>
							report("Running RTS " & to_hstring(subcycle));
							case subcycle(2 downto 0) is
								when "001" =>
								when "010" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
									pc <= pc(15 downto 8) & din;
								when "011" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
									pc <= din & pc(7 downto 0);
								when "100" =>
								when others =>
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"08" | x"48" =>
							case subcycle(2 downto 0) is
								when "001" =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) - "00000001");
								when others =>
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"28" =>
							case subcycle(2 downto 0) is
								when "001" =>
								when "010" =>
								when others =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
									flags <= din;
									flags(FLAG_BREAK) <= '0';
									flags(FLAG_UNUSED) <= '1';
									pc <= next_pc;
									instruction_toggle_pre <= not instruction_toggle_pre;
									subcycle <= (others => '0');
									opcode(8) <= '0';
							end case;
						when x"68" =>
							case subcycle(2 downto 0) is
								when "001" =>
								when "010" =>
								when others =>
									sp <= std_logic_vector(unsigned(sp(7 downto 0)) + "00000001");
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
						when x"10" =>
							case subcycle(2 downto 0) is
								when "001" =>
									if flags(FLAG_NEGATIVE) = '1' then
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									else
										pc <= addr_calc1;
										addr_calc2 <= addr_calc1;
									end if;
								when "010" =>
									pc <= next_pc;
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
						when x"30" =>
							case subcycle(2 downto 0) is
								when "001" =>
									if flags(FLAG_NEGATIVE) = '0' then
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									else
										pc <= addr_calc1;
										addr_calc2 <= addr_calc1;
									end if;
								when "010" =>
									pc <= next_pc;
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
						when x"50" =>
							case subcycle(2 downto 0) is
								when "001" =>
									if flags(FLAG_OVERFLOW) = '1' then
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									else
										pc <= addr_calc1;
										addr_calc2 <= addr_calc1;
									end if;
								when "010" =>
									pc <= next_pc;
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
						when x"70" =>
							case subcycle(2 downto 0) is
								when "001" =>
									if flags(FLAG_OVERFLOW) = '0' then
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									else
										pc <= addr_calc1;
										addr_calc2 <= addr_calc1;
									end if;
								when "010" =>
									pc <= next_pc;
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
						when x"90" =>
							case subcycle(2 downto 0) is
								when "001" =>
									if flags(FLAG_CARRY) = '1' then
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									else
										pc <= addr_calc1;
										addr_calc2 <= addr_calc1;
									end if;
								when "010" =>
									pc <= next_pc;
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
						when x"b0" =>
							case subcycle(2 downto 0) is
								when "001" =>
									if flags(FLAG_CARRY) = '0' then
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									else
										pc <= addr_calc1;
										addr_calc2 <= addr_calc1;
									end if;
								when "010" =>
									pc <= next_pc;
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
						when x"d0" =>
							case subcycle(2 downto 0) is
								when "001" =>
									if flags(FLAG_ZERO) = '1' then
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									else
										pc <= addr_calc1;
										addr_calc2 <= addr_calc1;
									end if;
								when "010" =>
									pc <= next_pc;
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
						when x"f0" =>
							case subcycle(2 downto 0) is
								when "001" =>
									if flags(FLAG_ZERO) = '0' then
										pc <= next_pc;
										instruction_toggle_pre <= not instruction_toggle_pre;
										subcycle <= (others => '0');
										opcode(8) <= '0';
									else
										pc <= addr_calc1;
										addr_calc2 <= addr_calc1;
									end if;
								when "010" =>
									pc <= next_pc;
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

