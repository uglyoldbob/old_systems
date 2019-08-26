/*
 * cartridge.c
 *
 * Created: 2/10/2017 11:05:22 AM
 *  Author: tepperson
 */ 

#include "cartridge.h"
#include "mappers/mappers.h"
#include "processor_6502.h"
#include "../fatfs/src/ff.h"
#include "../sam3u_includes.h"
#include "../uart.h"

void process_old_ines(FIL* fp)
{
	uart_send("old ines file\r\n");
}

uint8_t read_byte(FIL *fp, uint16_t offset)
{
	uint8_t retval;
	UINT result;
	f_lseek(fp, offset);
	f_read(fp, &retval, 1, &result);
	return retval;
}

void process_ines(FIL* fp)
{
	uint8_t prg_rom_size = read_byte(fp, 4);
	uint8_t chr_rom_size = read_byte(fp, 5);
	uint8_t flags6 = read_byte(fp, 6);
	uint8_t flags7 = read_byte(fp, 7);
	uint8_t prg_ram_size = read_byte(fp, 8);
	uint8_t flags9 = read_byte(fp, 9);
	uint8_t flags10 = read_byte(fp, 10);
	uint8_t mapper = (flags7&0xF0) | (flags6>>4);
	uart_send("ines file\r\n");
	uart_send("mapper ");
	uart_print(mapper);
	uart_send("\r\nPRG ROM: ");
	uart_print(prg_rom_size);
	uart_send("\r\nCHR ROM: ");
	uart_print(chr_rom_size);
	uart_send("\r\nPRG RAM: ");
	uart_print(prg_ram_size);
	uart_send("\r\n");
	switch (mapper)
	{
		case 0:
			set_mapper_cpu(mapper0_read, mapper0_write);
			uint32_t offset;
			if (flags6 & 4)
			{
				for (int i = 0; i < 256; i++)
				{
					SMC_CS0_AREA16[i] = read_byte(fp, 16+2*i)<<8 | read_byte(fp, 17+2*i);
				}
				offset = 518;
			}
			else
			{
				offset = 16;
			}
			for (volatile uint32_t i = 0; i < (prg_rom_size * 0x2000); i++)
			{
				volatile uint16_t temp = read_byte(fp, offset+2*i) | read_byte(fp, offset+1+2*i)<<8;
				SMC_CS0_AREA16[256+i] = temp;
			}
			mapper0_set_pgr_rom((uint8_t*)(&SMC_CS0_AREA16[256]), prg_rom_size * 0x4000);
			break;
		default:
			uart_send("Unsupported mapper: ");
			uart_print(mapper);
			uart_send("\r\n");
			break;
	}
}

void process_nes2(FIL* fp)
{
	uart_send("nes2 file\r\n");
}

uint8_t nes_rom_test(char *name)
{

	return 1;
}

void load_cartridge_image(char *img)
{
	FIL fp;
	f_open(&fp, img, FA_READ);
	if ( (read_byte(&fp, 0) == 0x4E) &&
		 (read_byte(&fp, 1) == 0x45) &&
		 (read_byte(&fp, 2) == 0x53) &&
		 (read_byte(&fp, 3) == 0x1a) )
	{

		if ((read_byte(&fp, 7) & 0xC) == 8)
		{
			process_nes2(&fp);
		}
		else if ( ((read_byte(&fp, 7) & 0xC) == 0) &&
		(read_byte(&fp, 12) == 0) &&
		(read_byte(&fp, 13) == 0) &&
		(read_byte(&fp, 14) == 0) &&
		(read_byte(&fp, 15) == 0))
		{
			process_ines(&fp);
		}
		else
		{
			process_old_ines(&fp);
		}
	}
}

void use_actual_cartridge(void)
{
	
}