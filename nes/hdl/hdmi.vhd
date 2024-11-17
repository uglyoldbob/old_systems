library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;
entity tmds_encoderb is
	Port(
		clock: in std_logic;
		cin: in std_logic_vector(7 downto 0);
		control: in std_logic_vector(1 downto 0);
		aux: in std_logic_vector(3 downto 0);
		sel: in std_logic_vector(1 downto 0);
		output: out std_logic_vector(9 downto 0));
end tmds_encoderb;

architecture Behavioral of tmds_encoderb is
	type LOOKUP is array (511 downto 0) of integer range 0 to 9;
	--below is generated with a program, i'm not that crazy
	signal ones_count : LOOKUP := (
		0, --000000000
		1, --000000001
		1, --000000010
		2, --000000011
		1, --000000100
		2, --000000101
		2, --000000110
		3, --000000111
		1, --000001000
		2, --000001001
		2, --000001010
		3, --000001011
		2, --000001100
		3, --000001101
		3, --000001110
		4, --000001111
		1, --000010000
		2, --000010001
		2, --000010010
		3, --000010011
		2, --000010100
		3, --000010101
		3, --000010110
		4, --000010111
		2, --000011000
		3, --000011001
		3, --000011010
		4, --000011011
		3, --000011100
		4, --000011101
		4, --000011110
		5, --000011111
		1, --000100000
		2, --000100001
		2, --000100010
		3, --000100011
		2, --000100100
		3, --000100101
		3, --000100110
		4, --000100111
		2, --000101000
		3, --000101001
		3, --000101010
		4, --000101011
		3, --000101100
		4, --000101101
		4, --000101110
		5, --000101111
		2, --000110000
		3, --000110001
		3, --000110010
		4, --000110011
		3, --000110100
		4, --000110101
		4, --000110110
		5, --000110111
		3, --000111000
		4, --000111001
		4, --000111010
		5, --000111011
		4, --000111100
		5, --000111101
		5, --000111110
		6, --000111111
		1, --001000000
		2, --001000001
		2, --001000010
		3, --001000011
		2, --001000100
		3, --001000101
		3, --001000110
		4, --001000111
		2, --001001000
		3, --001001001
		3, --001001010
		4, --001001011
		3, --001001100
		4, --001001101
		4, --001001110
		5, --001001111
		2, --001010000
		3, --001010001
		3, --001010010
		4, --001010011
		3, --001010100
		4, --001010101
		4, --001010110
		5, --001010111
		3, --001011000
		4, --001011001
		4, --001011010
		5, --001011011
		4, --001011100
		5, --001011101
		5, --001011110
		6, --001011111
		2, --001100000
		3, --001100001
		3, --001100010
		4, --001100011
		3, --001100100
		4, --001100101
		4, --001100110
		5, --001100111
		3, --001101000
		4, --001101001
		4, --001101010
		5, --001101011
		4, --001101100
		5, --001101101
		5, --001101110
		6, --001101111
		3, --001110000
		4, --001110001
		4, --001110010
		5, --001110011
		4, --001110100
		5, --001110101
		5, --001110110
		6, --001110111
		4, --001111000
		5, --001111001
		5, --001111010
		6, --001111011
		5, --001111100
		6, --001111101
		6, --001111110
		7, --001111111
		1, --010000000
		2, --010000001
		2, --010000010
		3, --010000011
		2, --010000100
		3, --010000101
		3, --010000110
		4, --010000111
		2, --010001000
		3, --010001001
		3, --010001010
		4, --010001011
		3, --010001100
		4, --010001101
		4, --010001110
		5, --010001111
		2, --010010000
		3, --010010001
		3, --010010010
		4, --010010011
		3, --010010100
		4, --010010101
		4, --010010110
		5, --010010111
		3, --010011000
		4, --010011001
		4, --010011010
		5, --010011011
		4, --010011100
		5, --010011101
		5, --010011110
		6, --010011111
		2, --010100000
		3, --010100001
		3, --010100010
		4, --010100011
		3, --010100100
		4, --010100101
		4, --010100110
		5, --010100111
		3, --010101000
		4, --010101001
		4, --010101010
		5, --010101011
		4, --010101100
		5, --010101101
		5, --010101110
		6, --010101111
		3, --010110000
		4, --010110001
		4, --010110010
		5, --010110011
		4, --010110100
		5, --010110101
		5, --010110110
		6, --010110111
		4, --010111000
		5, --010111001
		5, --010111010
		6, --010111011
		5, --010111100
		6, --010111101
		6, --010111110
		7, --010111111
		2, --011000000
		3, --011000001
		3, --011000010
		4, --011000011
		3, --011000100
		4, --011000101
		4, --011000110
		5, --011000111
		3, --011001000
		4, --011001001
		4, --011001010
		5, --011001011
		4, --011001100
		5, --011001101
		5, --011001110
		6, --011001111
		3, --011010000
		4, --011010001
		4, --011010010
		5, --011010011
		4, --011010100
		5, --011010101
		5, --011010110
		6, --011010111
		4, --011011000
		5, --011011001
		5, --011011010
		6, --011011011
		5, --011011100
		6, --011011101
		6, --011011110
		7, --011011111
		3, --011100000
		4, --011100001
		4, --011100010
		5, --011100011
		4, --011100100
		5, --011100101
		5, --011100110
		6, --011100111
		4, --011101000
		5, --011101001
		5, --011101010
		6, --011101011
		5, --011101100
		6, --011101101
		6, --011101110
		7, --011101111
		4, --011110000
		5, --011110001
		5, --011110010
		6, --011110011
		5, --011110100
		6, --011110101
		6, --011110110
		7, --011110111
		5, --011111000
		6, --011111001
		6, --011111010
		7, --011111011
		6, --011111100
		7, --011111101
		7, --011111110
		8, --011111111
		1, --100000000
		2, --100000001
		2, --100000010
		3, --100000011
		2, --100000100
		3, --100000101
		3, --100000110
		4, --100000111
		2, --100001000
		3, --100001001
		3, --100001010
		4, --100001011
		3, --100001100
		4, --100001101
		4, --100001110
		5, --100001111
		2, --100010000
		3, --100010001
		3, --100010010
		4, --100010011
		3, --100010100
		4, --100010101
		4, --100010110
		5, --100010111
		3, --100011000
		4, --100011001
		4, --100011010
		5, --100011011
		4, --100011100
		5, --100011101
		5, --100011110
		6, --100011111
		2, --100100000
		3, --100100001
		3, --100100010
		4, --100100011
		3, --100100100
		4, --100100101
		4, --100100110
		5, --100100111
		3, --100101000
		4, --100101001
		4, --100101010
		5, --100101011
		4, --100101100
		5, --100101101
		5, --100101110
		6, --100101111
		3, --100110000
		4, --100110001
		4, --100110010
		5, --100110011
		4, --100110100
		5, --100110101
		5, --100110110
		6, --100110111
		4, --100111000
		5, --100111001
		5, --100111010
		6, --100111011
		5, --100111100
		6, --100111101
		6, --100111110
		7, --100111111
		2, --101000000
		3, --101000001
		3, --101000010
		4, --101000011
		3, --101000100
		4, --101000101
		4, --101000110
		5, --101000111
		3, --101001000
		4, --101001001
		4, --101001010
		5, --101001011
		4, --101001100
		5, --101001101
		5, --101001110
		6, --101001111
		3, --101010000
		4, --101010001
		4, --101010010
		5, --101010011
		4, --101010100
		5, --101010101
		5, --101010110
		6, --101010111
		4, --101011000
		5, --101011001
		5, --101011010
		6, --101011011
		5, --101011100
		6, --101011101
		6, --101011110
		7, --101011111
		3, --101100000
		4, --101100001
		4, --101100010
		5, --101100011
		4, --101100100
		5, --101100101
		5, --101100110
		6, --101100111
		4, --101101000
		5, --101101001
		5, --101101010
		6, --101101011
		5, --101101100
		6, --101101101
		6, --101101110
		7, --101101111
		4, --101110000
		5, --101110001
		5, --101110010
		6, --101110011
		5, --101110100
		6, --101110101
		6, --101110110
		7, --101110111
		5, --101111000
		6, --101111001
		6, --101111010
		7, --101111011
		6, --101111100
		7, --101111101
		7, --101111110
		8, --101111111
		2, --110000000
		3, --110000001
		3, --110000010
		4, --110000011
		3, --110000100
		4, --110000101
		4, --110000110
		5, --110000111
		3, --110001000
		4, --110001001
		4, --110001010
		5, --110001011
		4, --110001100
		5, --110001101
		5, --110001110
		6, --110001111
		3, --110010000
		4, --110010001
		4, --110010010
		5, --110010011
		4, --110010100
		5, --110010101
		5, --110010110
		6, --110010111
		4, --110011000
		5, --110011001
		5, --110011010
		6, --110011011
		5, --110011100
		6, --110011101
		6, --110011110
		7, --110011111
		3, --110100000
		4, --110100001
		4, --110100010
		5, --110100011
		4, --110100100
		5, --110100101
		5, --110100110
		6, --110100111
		4, --110101000
		5, --110101001
		5, --110101010
		6, --110101011
		5, --110101100
		6, --110101101
		6, --110101110
		7, --110101111
		4, --110110000
		5, --110110001
		5, --110110010
		6, --110110011
		5, --110110100
		6, --110110101
		6, --110110110
		7, --110110111
		5, --110111000
		6, --110111001
		6, --110111010
		7, --110111011
		6, --110111100
		7, --110111101
		7, --110111110
		8, --110111111
		3, --111000000
		4, --111000001
		4, --111000010
		5, --111000011
		4, --111000100
		5, --111000101
		5, --111000110
		6, --111000111
		4, --111001000
		5, --111001001
		5, --111001010
		6, --111001011
		5, --111001100
		6, --111001101
		6, --111001110
		7, --111001111
		4, --111010000
		5, --111010001
		5, --111010010
		6, --111010011
		5, --111010100
		6, --111010101
		6, --111010110
		7, --111010111
		5, --111011000
		6, --111011001
		6, --111011010
		7, --111011011
		6, --111011100
		7, --111011101
		7, --111011110
		8, --111011111
		4, --111100000
		5, --111100001
		5, --111100010
		6, --111100011
		5, --111100100
		6, --111100101
		6, --111100110
		7, --111100111
		5, --111101000
		6, --111101001
		6, --111101010
		7, --111101011
		6, --111101100
		7, --111101101
		7, --111101110
		8, --111101111
		5, --111110000
		6, --111110001
		6, --111110010
		7, --111110011
		6, --111110100
		7, --111110101
		7, --111110110
		8, --111110111
		6, --111111000
		7, --111111001
		7, --111111010
		8, --111111011
		7, --111111100
		8, --111111101
		8, --111111110
		9  --111111111
	);
	
	signal ones_count_cin: integer range 0 to 9;
	signal ones_count_qm: integer range 0 to 9;
	signal q_m: std_logic_vector(8 downto 0) := (others => '0');
	
	signal disparity: integer range -16 to 15;
begin
	process (all)
	begin
		ones_count_cin <= ones_count(to_integer(unsigned('0' & cin)));
		ones_count_qm <= ones_count(to_integer(unsigned(q_m)));
	end process;
	
	process (clock)
	begin
		if rising_edge(clock) then
			if ones_count_cin > 4 or ((ones_count_cin = 4) and cin(0) = '0') then
				q_m(0) <= cin(0);
				q_m(7 downto 1) <= q_m(7 downto 1) xnor cin(7 downto 1);
				q_m(8) <= '0';
			else
				q_m(0) <= cin(0);
				q_m(7 downto 1) <= q_m(7 downto 1) xor cin(7 downto 1);
				q_m(8) <= '1';
			end if;
		end if;
		if rising_edge(clock) then
			case sel is
				when "00" =>
					if disparity = 0 or ones_count_qm = 4 then
						if not q_m(8) then
							output <= not q_m(8) & q_m(8) & not q_m(7 downto 0);
							disparity <= disparity + 9 - ones_count_qm - ones_count_qm;
						else
							output <= not q_m(8) & q_m(8) & q_m(7 downto 0);
							disparity <= disparity - 9 + ones_count_qm + ones_count_qm;
						end if;
					else
						if ((disparity > 0 and ones_count_qm > 4) or (disparity < 0 and ones_count_qm < 4)) then
							output <= '1' & q_m(8) & not q_m(7 downto 0);
							if not q_m(8) then
								disparity <= disparity + 9 - ones_count_qm - ones_count_qm;
							else
								disparity <= disparity + 11 - ones_count_qm - ones_count_qm;
							end if;
						else
							output <= '0' & q_m(8) & q_m(7 downto 0);
							if not q_m(8) then
								disparity <= disparity - 11 + ones_count_qm + ones_count_qm;
							else
								disparity <= disparity - 9 + ones_count_qm + ones_count_qm;
							end if;
						end if;
					end if;
				when "01" =>
					disparity <= 0;
					case control is
						when "00" => output <= "1101010100";
						when "01" => output <= "0010101011";
						when "10" => output <= "0101010100";
						when others => output <= "1010101011";
					end case;
				when "10" =>
					disparity <= 0;
					case aux is
						when "0000" => output <= "1010011100";
						when "0001" => output <= "1001100011";
						when "0010" => output <= "1011100100";
						when "0011" => output <= "1011100010";
						when "0100" => output <= "0101110001";
						when "0101" => output <= "0100011110";
						when "0110" => output <= "0110001110";
						when "0111" => output <= "0100111100";
						when "1000" => output <= "1011001100";
						when "1001" => output <= "0100111001";
						when "1010" => output <= "0110011100";
						when "1011" => output <= "1011000110";
						when "1100" => output <= "1010001110";
						when "1101" => output <= "1001110001";
						when "1110" => output <= "0101100011";
						when others => output <= "1011000011";
					end case;
				when others => output <= "1111100000";
			end case;
		end if;
	end process;
	
end Behavioral;

library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity tmds_multiplexer is
	Port(
		clock: in std_logic;
		pixel_clock: in std_logic;
		reset: in std_logic;
		din: in std_logic_vector(9 downto 0);
		dout: out std_logic_vector(1 downto 0));
end tmds_multiplexer;

architecture Behavioral of tmds_multiplexer is
signal counter: std_logic_vector(2 downto 0) := "000";
begin

	process (all)
	begin
		case counter is
			when "000" => dout <= din(1 downto 0);
			when "001" => dout <= din(3 downto 2);
			when "010" => dout <= din(5 downto 4);
			when "011" => dout <= din(7 downto 6);
			when others => dout <= din(9 downto 8);
		end case;
	end process;

	process (reset, clock)
	begin
		if rising_edge(clock) then
			if reset then
				counter <= "000";
			else
				counter <= std_logic_vector(unsigned(counter) + 1);
				if counter = "100" then
					counter <= "000";
				end if;
			end if;
		end if;
	end process;
end Behavioral;

library IEEE;
use IEEE.STD_LOGIC_1164.ALL;
use IEEE.NUMERIC_STD.ALL;

entity hdmi is
	Generic(h: integer := 1920;
        v: integer := 1080;
        hblank_width: integer := 280;
        vblank_width: integer := 45;
		  hsync_polarity: std_logic := '1';
		  hsync_porch: integer := 148;
		  hsync_width: integer := 44;
		  vsync_width: integer := 5;
		  vsync_porch: integer := 36;
		  vsync_polarity: std_logic := '1';
        rate: integer := 60);
	Port(
		reset: in std_logic;
		clock_freq: out integer;
		pixel_clock: in std_logic;
		tmds_clock: in std_logic;
		tmds_0: out std_logic_vector(9 downto 0);
		tmds_1: out std_logic_vector(9 downto 0);
		tmds_2: out std_logic_vector(9 downto 0);
		ck_p: out std_logic;
		ck_n: out std_logic;
		cec: inout std_logic;
		i2c_scl: inout std_logic;
		i2c_sda: inout std_logic;
		hpd: inout std_logic;
		vsync_out: out std_logic;
		r: in std_logic_vector(7 downto 0);
		g: in std_logic_vector(7 downto 0);
		b: in std_logic_vector(7 downto 0));
end hdmi;

architecture Behavioral of hdmi is
	signal control: std_logic_vector(3 downto 0) := (others => '0');
	signal aux: std_logic_vector(3 downto 0) := (others => '0');
	signal aux2: std_logic_vector(3 downto 0) := (others => '0');
	signal aux3: std_logic_vector(3 downto 0) := (others => '0');
	
	signal tmds_0_pre: std_logic_vector(9 downto 0);
	signal tmds_1_pre: std_logic_vector(9 downto 0);
	signal tmds_2_pre: std_logic_vector(9 downto 0);
	
	signal htotal: integer := (h + hblank_width);
	signal vtotal: integer := (v + vblank_width);
	
	signal column: integer range 0 to h+hblank_width-1 := 0;
	signal row: integer range 0 to v+vblank_width-1:= 0;
	
	signal hblank: std_logic;
	signal vblank: std_logic;
	signal hsync: std_logic;
	signal vsync: std_logic;
	signal control_period: std_logic;
	signal pixels_guard: std_logic;
	signal pixels: std_logic;
	signal data_island_guard: std_logic;
	signal data_island: std_logic;
	signal data_island_preamble: std_logic;
	signal pixel_preamble: std_logic;
	
	signal request_data_island: std_logic;
	signal data_island_mode: std_logic_vector(2 downto 0) := (others => '0');
	signal data_island_guard_count: std_logic := '0';
	signal data_island_counter: std_logic_vector(9 downto 0) := (others => '0');
	
	signal selection: std_logic_vector(1 downto 0);
begin
    clock_freq <= htotal * vtotal * rate;

    process (all)
    begin
		if vblank then
			vsync_out <= '1';
		else
			vsync_out <= '0';
		end if;
    end process;

	data_island <= '0'; --todo
	data_island_guard <= '0';--data_island_mode(1);
	 data_island_preamble <= '0';--data_island_mode(2);
	 process (pixel_clock)
	 begin
		if rising_edge(pixel_clock) then
			case data_island_mode is
				when "000" =>
					if request_data_island then
						data_island_mode <= "100";
					end if;
				when "100" =>
					data_island_counter <= std_logic_vector(unsigned(data_island_counter) + 1);
					if data_island_counter(4 downto 0) = "00111" then
						data_island_mode <= "010";
						data_island_counter <= (others => '0');
					end if;
				when "010" =>
					data_island_guard_count <= not data_island_guard_count;
					if data_island_guard_count then
						data_island_mode <= "001";
					end if;
				when "011" =>
					data_island_guard_count <= not data_island_guard_count;
					if data_island_guard_count then
						data_island_mode <= "000";
					end if;
				when others =>
					data_island_counter <= std_logic_vector(unsigned(data_island_counter) + 1);
					if data_island_counter(4 downto 0) = "11111" then
						data_island_mode <= "011";
						data_island_counter <= (others => '0');
					end if;
			end case;
		end if;
	 end process;
	 
	 process (all)
	 begin
		if column = 15 then
			request_data_island <= '0';
		else
			request_data_island <= '0';
		end if;
		if column = (hsync_porch + hsync_width-1) or column = (hsync_porch + hsync_width-2) then
			pixels_guard <= '1';
		else
			pixels_guard <= '0';
		end if;
		if column < hsync_width then
			hsync <= hsync_polarity;
		else
			hsync <= not hsync_polarity;
		end if;
		if row < vsync_width then
			vsync <= vsync_polarity;
		else
			vsync <= not vsync_polarity;
		end if;
		if column >= (hsync_porch + hsync_width-10) and column <= (hsync_porch + hsync_width - 3) then
			pixel_preamble <= '1';
		else
			pixel_preamble <= '0';
		end if;
		if column < (hsync_width + hsync_porch) or column >= (hsync_width + hsync_porch + h) then
			hblank <= '1';
		else
			hblank <= '0';
		end if;
		if row < (vsync_width + vsync_porch) or row >= (vsync_width + vsync_porch + v) then
			vblank <= '1';
		else
			vblank <= '0';
		end if;
		pixels <= (hblank or pixels_guard) nor vblank;
	 end process;
	 
	 process (pixel_clock)
	 begin
		if rising_edge(pixel_clock) then
			if column = (htotal - 1) then
				column <= 0;
				if row = (vtotal - 1) then
					row <= 0;
				else
					row <= row + 1;
				end if;
			else
				column <= column + 1;
			end if;
			if pixels then
				selection <= "00";  --normal pixel output
			elsif data_island then
				selection <= "10";  --aux output
			elsif pixels_guard or hblank or vblank then
				selection <= "01";  --ctl output
			else
				selection <= "11";
			end if;
		end if;
	 end process;

	d0_encoder: entity work.tmds_encoderb port map(
		clock => pixel_clock,
		cin => r,
		control => hsync & vsync,
		aux => aux,
		sel => selection,
		output => tmds_0_pre
	);
	
	process (pixel_clock)
	begin
		if rising_edge(pixel_clock) then
			if data_island_preamble then
				control <= "0101";
			elsif pixel_preamble then
				control <= "0001";
			else
				control <= "0000";
			end if;
		end if;
	end process;
	
	process (pixel_clock)
	begin
        if rising_edge(pixel_clock) then
            if pixels_guard then
                tmds_0 <= "1011001100";
                tmds_1 <= "0100110011";
                tmds_2 <= "1011001100";
            elsif data_island_guard then
                tmds_0 <= tmds_0_pre;
                tmds_1 <= "0100110011";
                tmds_2 <= "0100110011";
            else
                tmds_0 <= tmds_0_pre;
                tmds_1 <= tmds_1_pre;
                tmds_2 <= tmds_2_pre;
            end if;
        end if;
	end process;
	
	d1_encoder: entity work.tmds_encoderb port map(
		clock => pixel_clock,
		cin => g,
		control => control(1 downto 0),
		aux => aux2,
		sel => selection,
		output => tmds_1_pre
	);
	
	d2_encoder: entity work.tmds_encoderb port map(
		clock => pixel_clock,
		cin => b,
		control => control(3 downto 2),
		aux => aux3,
		sel => selection,
		output => tmds_2_pre
	);
	
	ck_p <= pixel_clock;
	ck_n <= not pixel_clock;

	process(pixel_clock)
	begin
		if rising_edge(pixel_clock) then	
		end if;
	end process;
end Behavioral;