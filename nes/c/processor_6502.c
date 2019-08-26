/*
 * processor_6502.c
 *
 * Created: 2/7/2017 1:21:29 PM
 *  Author: tepperson
 */ 

#include <stdint.h>

#include "processor_6502.h"
#include "../uart.h"

static uint8_t internal_ram[2048];

static uint8_t cur_instr[5];
static uint8_t cur_instr_index = 0;

uint8_t p6502_single_step = 0;
static uint8_t hang = 1;
static uint32_t cycle_number = 0;

static uint8_t reg_a;
static uint8_t reg_x, reg_y;
static uint16_t pc;
static uint8_t reg_s;	//stack pointer
static uint8_t reg_p;	//status register
#define STATUS_CARRY 1
#define STATUS_ZERO 2
#define STATUS_INTERRUPT_DISABLE 4
#define STATUS_DECIMAL 8
#define STATUS_BREAK 0x10
#define STATUS_EXPAND 0x20
#define STATUS_OVERFLOW 0x40
#define STATUS_NEGATIVE 0x80

#define RESET_VECTOR 0xFFFC

#define CPU_DEBUG 1
//#undef CPU_DEBUG

static mem_read_callback mapper_cpur = 0;
static mem_write_callback mapper_cpuw = 0;
static mem_read_callback mapper_ppur = 0;
static mem_write_callback mapper_ppuw = 0;

uint8_t ppu_read(uint16_t addr)
{
	return 0;
}

uint8_t apu_read(uint16_t addr)
{
	return 0;
}

uint8_t mapper_read(uint16_t addr)
{
	if (mapper_cpur != 0)
	{
		return (mapper_cpur)(addr);
	}
	else
	{
		return 0;
	}
}

void set_mapper_cpu(mem_read_callback r, mem_write_callback w)
{
	mapper_cpur = r;
	mapper_cpuw = w;
}

void set_mapper_ppu(mem_read_callback r, mem_write_callback w)
{
	mapper_ppur = r;
	mapper_ppuw = w;
}


uint8_t read_memory(uint16_t addr)
{
	#ifdef CPU_DEBUG
	uart_send("Read value [");
	uart_printhex(addr);
	uart_send("] ");
	#endif
	uint8_t retval;
	if (addr < 0x2000)
	{
		retval = internal_ram[addr & 0x7FF];
	}
	else if (addr < 0x4000)
	{
		retval = ppu_read(addr & 0x2007);
	}
	else if (addr < 0x4018)
	{
		retval = apu_read(addr);
	}
	else if (addr < 0x4020)
	{
		retval = 0;
	}
	else
	{
		retval = mapper_read(addr);
	}
	#ifdef CPU_DEBUG
	uart_printhex(retval);
	uart_send("\r\n");
	#endif
	return retval;
}

void write_memory(uint16_t addr, uint8_t val)
{
	#ifdef CPU_DEBUG
	uart_send("Write value [");
	uart_printhex(addr);
	uart_send("] ");
	uart_printhex(val);
	uart_send("\r\n");
	#endif
	if (addr < 0x2000)
	{
		internal_ram[addr & 0x7FF] = val;
	}
	else if (addr < 0x4000)
	{
		//ppu_write(addr & 0x2007, val);
	}
	else if (addr < 0x4018)
	{
		//apu_write(addr, val);
	}
	else if (addr < 0x4020)
	{
		
	}
	else
	{
		//mapper_write(addr, val);
	}
	
}

void p6502_execute_vector(uint16_t vector_addr)
{
	if (vector_addr == RESET_VECTOR)
	{
		read_memory(pc);
		read_memory(pc+1);
		read_memory(0x100 + reg_s);
		read_memory(0xFF + reg_s);
		read_memory(0xFE + reg_s);
	}
	else
	{
		read_memory(pc);
		read_memory(pc);	//pc+1?
		write_memory(reg_s--, pc>>8);
		write_memory(reg_s--, pc&0xFF);
		write_memory(reg_s--, reg_p | STATUS_BREAK);
	}
	uint16_t temp = read_memory(vector_addr+1)<<8 | read_memory(vector_addr);
	pc = temp;
	#ifdef CPU_DEBUG
	uart_send("Boot to ");
	uart_printhex(pc);
	uart_send("\r\n");
	#endif
}

uint8_t instr_length()
{
	switch(cur_instr[0])
	{
		case 0x78:
			return 1;
		default:
			return 0;
	}
}

void fetch_opcode()
{
	uint8_t val = read_memory(pc++);
	cur_instr[cur_instr_index++] = val;
}

void branch_flag(uint8_t mask, uint8_t cmp)
{
	if (cur_instr_index < 2)
	{
		#ifdef CPU_DEBUG
		switch (cur_instr[0])
		{
			case 0x10:
				uart_send("BPL ");
				break;
			case 0x30:
				uart_send("BMI ");
				break;
			case 0x50:
				uart_send("BVC ");
				break;
			case 0x70:
				uart_send("BVS ");
				break;
			case 0x90:
				uart_send("BCC ");
				break;
			case 0xB0:
				uart_send("BCS ");
				break;
			case 0xD0:
				uart_send("BNE ");
				break;
			case 0xF0:
				uart_send("BEQ ");
				break;
			default:
				break;
		}
		#endif
		fetch_opcode();
	}
	if (cur_instr_index == 2)
	{
		if ((reg_p & mask) != cmp)
		{
			#ifdef CPU_DEBUG
			uart_send("Not branching\r\n");
			#endif
			cur_instr_index = 0;
		}
		else
		{
			cur_instr_index++;
		}
	}
	else if (cur_instr_index == 3)
	{
		#ifdef CPU_DEBUG
		uart_send("Branching\r\n");
		#endif
		uint16_t calc = (char)cur_instr[1];
		if (calc & 0x80)
			calc |= 0xFF00;
		uint16_t newpc = pc + calc;
		if ((newpc>>8) != (pc>>8))
		{
			#ifdef CPU_DEBUG
			uart_send("Cross page boundary Branch to ");
			uart_printhex(newpc);
			uart_send("\r\n");
			#endif
			pc = newpc;
			cur_instr_index = 0;
		}
		else
		{
			#ifdef CPU_DEBUG
			uart_send("Branch to ");
			uart_printhex(newpc);
			uart_send("\r\n");
			#endif
			pc = newpc;
			cur_instr_index = 0;
		}
	}
	else
	{
		
	}
}

void do_adc(uint8_t value)
{
	uint16_t calc = reg_a + value + ((reg_p&STATUS_CARRY)?1:0);
	reg_p = (reg_p & ~STATUS_CARRY) | ((calc&0xFF00)?STATUS_CARRY:0);
	uint8_t temp2 = (((reg_a & value & ~calc) | (~reg_a & ~value & calc))) & 0x80;
	reg_a = calc&0xFF;
	reg_p = (reg_p & ~STATUS_OVERFLOW) | ((temp2)?STATUS_OVERFLOW:0);
	reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a&0x80)?STATUS_NEGATIVE:0);
	reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
}

void do_sbc(uint8_t value)
{
	uint8_t temp = reg_a - value - ((reg_p&STATUS_CARRY)?0:1);
	reg_p = (reg_p & ~STATUS_CARRY) | ((temp < reg_a)?STATUS_CARRY:0);
	uint8_t temp2 = ((reg_a & ~value & ~temp) | (~reg_a & value & temp)) & 0x80;
	reg_p = (reg_p & ~STATUS_OVERFLOW) | ((temp2)?STATUS_OVERFLOW:0);
	reg_p = (reg_p & ~STATUS_NEGATIVE) | ((temp&0x80)?STATUS_NEGATIVE:0);
	reg_p = (reg_p & ~STATUS_ZERO) | ((temp)?0:STATUS_ZERO);
	reg_a = temp&0xFF;
}

void do_cmp(uint8_t value)
{
	reg_p = (reg_p & ~STATUS_NEGATIVE) | (((reg_a-value)&0x80)?STATUS_NEGATIVE:0);
	reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a==value)?STATUS_ZERO:0);
	reg_p = (reg_p & ~STATUS_CARRY) | ((reg_a>=value)?STATUS_CARRY:0);
}

void decode_opcode()
{
	if ((cur_instr[0] & 0x3) == 0x1)
	{	//these instructions have all 8 addressing modes
		uint8_t ready = 0;
		#ifdef CPU_DEBUG
		switch (cur_instr[0] & 0xE0)
		{
			case 0x00: uart_send("ORA "); break;
			case 0x20: uart_send("AND "); break;
			case 0x40: uart_send("EOR "); break;
			case 0x60: uart_send("ADC "); break;
			case 0x80: uart_send("STA "); break;
			case 0xA0: uart_send("LDA "); break;
			case 0xC0: uart_send("CMP "); break;
			case 0xE0: uart_send("SBC "); break;
			default: break;
		}
		#endif
		switch (cur_instr[0] & 0x1D)
		{
			case 0x09: //immediate
				fetch_opcode();
				#ifdef CPU_DEBUG
				uart_send("immediate ");
				uart_printhex(cur_instr[1]);
				uart_send("\r\n");
				#endif
				cur_instr[4] = cur_instr[1];
				ready = 1;
				break;
			case 0x05: //zero page
				if (cur_instr_index == 1)
				{
					fetch_opcode();
					#ifdef CPU_DEBUG
					uart_send("zero page [");
					uart_printhex(cur_instr[1]);
					uart_send("]");
					#endif
					cur_instr[2] = 0;
					if ((cur_instr[0] & 0xE0) != 0x80)
						cur_instr[4] = read_memory(cur_instr[1]);
					#ifdef CPU_DEBUG
					uart_send(" (");
					uart_printhex(cur_instr[4]);
					uart_send(")\r\n");
					#endif
				}
				else if (cur_instr_index == 2)
				{
					ready = 1;
				}
				break;
			case 0x15: //zero page x
				if (cur_instr_index == 1)
				{
					fetch_opcode();
					#ifdef CPU_DEBUG
					uart_send("zero page x [");
					uart_printhex(0xFF&(reg_x+cur_instr[1]));
					uart_send("]");
					#endif
					cur_instr[1] = 0xFF&(reg_x+cur_instr[1]);
					cur_instr[2] = 0;
					if ((cur_instr[0] & 0xE0) != 0x80)
						cur_instr[4] = read_memory(cur_instr[1]);
					#ifdef CPU_DEBUG
					uart_send(" (");
					uart_printhex(cur_instr[4]);
					uart_send(")\r\n");
					#endif
				}
				else if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					ready = 1;
				}
				break;
			case 0x0D: //absolute
				#ifdef CPU_DEBUG
				uart_send("absolute [");
				#endif
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					uint16_t ea = cur_instr[1] | cur_instr[2]<<8;
					#ifdef CPU_DEBUG
					uart_printhex(ea);
					uart_send("] (");
					#endif
					if ((cur_instr[0] & 0xE0) != 0x80)
						cur_instr[4] = read_memory(ea);
					#ifdef CPU_DEBUG
					uart_printhex(cur_instr[4]);
					uart_send(")\r\n");
					#endif
					cur_instr_index++;
					ready = 1;
				}
				break;
			case 0x1D: //absolute x
				switch (cur_instr_index)
				{
					case 1:
						fetch_opcode();
						break;
					case 2:
						fetch_opcode();
						#ifdef CPU_DEBUG
						uart_send("absolute x \r\n");
						#endif
						break;
					case 3:
					{
						uint16_t ea = (reg_x  + cur_instr[1] + (cur_instr[2]<<8));
						if ( ((ea>>8) == cur_instr[2]) && ((cur_instr[0] & 0xE0) != 0x80) )
							ready = 1;
						cur_instr[1] = ea & 0xFF;
						cur_instr[2] = ea>>8;
						cur_instr[4] = read_memory(ea);
						cur_instr_index++;
						break;
					}
					default:
						cur_instr_index++;
						ready = 1;
						break;
				}
				break;
			case 0x19: //absolute y
				switch (cur_instr_index)
				{
					case 1:
						fetch_opcode();
						break;
					case 2:
						fetch_opcode();
						#ifdef CPU_DEBUG
						uart_send("absolute y \r\n");
						#endif
						break;
					case 3:
					{
						uint16_t ea = (reg_y  + cur_instr[1] + (cur_instr[2]<<8));
						if ( ((ea>>8) == cur_instr[2]) && ((cur_instr[0] & 0xE0) != 0x80) )
							ready = 1;
						cur_instr[1] = ea & 0xFF;
						cur_instr[2] = ea>>8;
						cur_instr[4] = read_memory(ea);
						cur_instr_index++;
						break;
					}
					default:
						cur_instr_index++;
						ready = 1;
						break;
				}				
				break;
			case 0x01: //indirect x
				if (cur_instr_index == 1)
				{
					fetch_opcode();
					#ifdef CPU_DEBUG
					uart_send("indirect x \r\n");
					#endif
				}
				else if (cur_instr_index == 2)
				{
					cur_instr_index++;
					cur_instr[2] = read_memory(0xFF&(cur_instr[1]+reg_x));
				}
				else if (cur_instr_index == 3)
				{
					cur_instr_index++;
					cur_instr[3] = read_memory(0xFF&(cur_instr[1]+reg_x+1));
				}
				else if (cur_instr_index == 4)
				{
					cur_instr[1] = cur_instr[2];
					cur_instr[2] = cur_instr[3];
					uint16_t ea = cur_instr[1] | cur_instr[2]<<8;
					if ((cur_instr[0] & 0xE0) != 0x80)
						cur_instr[4] = read_memory(ea);
					cur_instr_index++;
				}
				else
				{
					ready = 1;
				}
				break;
			case 0x11: //indirect y (2, 5*)
				if (cur_instr_index == 1)
				{
					fetch_opcode();
					#ifdef CPU_DEBUG
					uart_send("indirect y \r\n");
					#endif
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[3] = read_memory(cur_instr[1]);
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[2] = read_memory(0xFF&(cur_instr[1]+1));
					cur_instr[1] = cur_instr[3];
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					uint16_t ea = (cur_instr[1] + (cur_instr[2]<<8) + reg_y);
					if ((((ea & 0xFF00)>>8) != cur_instr[2]) | ((cur_instr[0] & 0xE0) == 0x80))
					{
						cur_instr[1] = ea&0xFF;
						cur_instr[2] = ea>>8;
						cur_instr_index++;
					}
					else
					{
						cur_instr[1] = ea&0xFF;
						cur_instr[2] = ea>>8;
						cur_instr_index = 6;
						if ((cur_instr[0] & 0xE0) != 0x80)
							cur_instr[4] = read_memory(cur_instr[1] + (cur_instr[2]<<8));
						ready = 1;
					}
				}
				else
				{
					if ((cur_instr[0] & 0xE0) != 0x80)
						cur_instr[4] = read_memory(cur_instr[1] + (cur_instr[2]<<8));
					ready = 1;
				}
				break;
			default:
				break;
		}
		if (ready == 1)
		{
			switch (cur_instr[0] & 0xE0)
			{
				case 0x00:
					reg_a |= cur_instr[4]; //nz
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
					break;
				case 0x20:
					reg_a &= cur_instr[4]; //nz
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
					break;
				case 0x40:
					reg_a ^= cur_instr[4]; //nz
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
					break;
				case 0x60:
					do_adc(cur_instr[4]);
					break;
				case 0x80:
					write_memory(cur_instr[1] | cur_instr[2]<<8, reg_a);	//no flags
					break;
				case 0xA0:
					reg_a = cur_instr[4]; //nz
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
					break;
				case 0xC0:
					do_cmp(cur_instr[4]);
					break;
				case 0xE0:
					do_sbc(cur_instr[4]);
					break;
				default: break;
			}
			cur_instr_index = 0;
		}
	}
	else
	{
		
		switch(cur_instr[0])
		{
			case 0xE0:	//cpx immediate
				fetch_opcode();
				#ifdef CPU_DEBUG
				uart_send("CPX ");
				uart_printhex(cur_instr[1]);
				uart_send("\r\n");
				#endif
				reg_p = (reg_p & ~STATUS_NEGATIVE) | (((reg_x-cur_instr[1])&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x==cur_instr[1])?STATUS_ZERO:0);
				reg_p = (reg_p & ~STATUS_CARRY) | ((reg_x>=cur_instr[1])?STATUS_CARRY:0);
				cur_instr_index = 0;
				break;
			case 0xE4:	//cpx zero page
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("CPX ");
					uart_printhex(cur_instr[1]);
					uart_send("\r\n");
					#endif
					cur_instr[1] = read_memory(cur_instr[1]);
					reg_p = (reg_p & ~STATUS_NEGATIVE) | (((reg_x-cur_instr[1])&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x==cur_instr[1])?STATUS_ZERO:0);
					reg_p = (reg_p & ~STATUS_CARRY) | ((reg_x>=cur_instr[1])?STATUS_CARRY:0);
					cur_instr_index = 0;
				}
				break;
			case 0xEC:	//cpx absolute
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("CPX ");
					uart_printhex(cur_instr[1] | (cur_instr[2]<<8));
					uart_send("\r\n");
					#endif
					cur_instr[1] = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					reg_p = (reg_p & ~STATUS_NEGATIVE) | (((reg_x-cur_instr[1])&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x==cur_instr[1])?STATUS_ZERO:0);
					reg_p = (reg_p & ~STATUS_CARRY) | ((reg_x>=cur_instr[1])?STATUS_CARRY:0);
					cur_instr_index = 0;
				}
				break;
			case 0xC0:	//cpy immediate
				fetch_opcode();
				#ifdef CPU_DEBUG
				uart_send("CPY ");
				uart_printhex(cur_instr[1]);
				uart_send("\r\n");
				#endif
				reg_p = (reg_p & ~STATUS_NEGATIVE) | (((reg_y-cur_instr[1])&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y==cur_instr[1])?STATUS_ZERO:0);
				reg_p = (reg_p & ~STATUS_CARRY) | ((reg_y>=cur_instr[1])?STATUS_CARRY:0);
				cur_instr_index = 0;
				break;
			case 0xC4:	//cpy zero page
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("CPY ");
					uart_printhex(cur_instr[1]);
					uart_send("\r\n");
					#endif
					cur_instr[1] = read_memory(cur_instr[1]);
					reg_p = (reg_p & ~STATUS_NEGATIVE) | (((reg_y-cur_instr[1])&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y==cur_instr[1])?STATUS_ZERO:0);
					reg_p = (reg_p & ~STATUS_CARRY) | ((reg_y>=cur_instr[1])?STATUS_CARRY:0);
					cur_instr_index = 0;
				}
				break;
			case 0xCC:	//cpy absolute
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("CPY ");
					uart_printhex(cur_instr[1] | (cur_instr[2]<<8));
					uart_send("\r\n");
					#endif
					cur_instr[1] = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					reg_p = (reg_p & ~STATUS_NEGATIVE) | (((reg_y-cur_instr[1])&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y==cur_instr[1])?STATUS_ZERO:0);
					reg_p = (reg_p & ~STATUS_CARRY) | ((reg_y>=cur_instr[1])?STATUS_CARRY:0);
					cur_instr_index = 0;
				}
				break;
			case 0xC6:	//decrement zero page
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(cur_instr[1]);
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("DEC \r\n");
					#endif
					cur_instr[2]--;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(cur_instr[1], cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0xD6:	//decrement zero page x
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(0xFF&(reg_x+cur_instr[1]));
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("DEC \r\n");
					#endif
					cur_instr[2]--;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(0xFF&(reg_x+cur_instr[1]), cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0xCE:	//decrement absolute
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("DEC \r\n");
					#endif
					cur_instr[3]--;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] | (cur_instr[2]<<8), cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0xDE:	//decrement absolute x
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x);
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 5)
				{
					#ifdef CPU_DEBUG
					uart_send("DEC \r\n");
					#endif
					cur_instr[3]--;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x, cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0xE6:	//increment zero page
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(cur_instr[1]);
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("INC \r\n");
					#endif
					cur_instr[2]++;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(cur_instr[1], cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0xF6:	//increment zero page x
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(0xFF&(reg_x+cur_instr[1]));
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("INC \r\n");
					#endif
					cur_instr[2]++;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(0xFF&(reg_x+cur_instr[1]), cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0xEE:	//increment absolute
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("INC \r\n");
					#endif
					cur_instr[3]++;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] | (cur_instr[2]<<8), cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0xFE:	//increment absolute x
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x);
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 5)
				{
					#ifdef CPU_DEBUG
					uart_send("INC \r\n");
					#endif
					cur_instr[3]++;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x, cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0xE8:	//increment X
				#ifdef CPU_DEBUG
				uart_send("INX\r\n");
				#endif
				reg_x++;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_x&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x)?0:STATUS_ZERO);
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0xC8:	//increment Y
				#ifdef CPU_DEBUG
				uart_send("INY\r\n");
				#endif
				reg_y++;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_y&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y)?0:STATUS_ZERO);
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0xCA:	//decrement X
				#ifdef CPU_DEBUG
				uart_send("DEX\r\n");
				#endif
				reg_x--;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_x&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x)?0:STATUS_ZERO);
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0x88:	//decrement Y
				#ifdef CPU_DEBUG
				uart_send("DEY\r\n");
				#endif
				reg_y--;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_y&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y)?0:STATUS_ZERO);
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0xAA:	//X = accumulator
				#ifdef CPU_DEBUG
				uart_send("TAX\r\n");
				#endif
				reg_x = reg_a;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_x&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x)?0:STATUS_ZERO);
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0xBA:	//X = s
				#ifdef CPU_DEBUG
				uart_send("TSX\r\n");
				#endif
				reg_x = reg_s;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_x&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x)?0:STATUS_ZERO);
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0x9A:	//s = X
				#ifdef CPU_DEBUG
				uart_send("TXS\r\n");
				#endif
				reg_s = reg_x;
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0xA8:	//Y = accumulator
				#ifdef CPU_DEBUG
				uart_send("TAY\r\n");
				#endif
				reg_y = reg_a;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_y&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y)?0:STATUS_ZERO);
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0x8A:	//accumulator = x
				#ifdef CPU_DEBUG
				uart_send("TXA\r\n");
				#endif
				reg_a = reg_x;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0x98:	//accumulator = y
				#ifdef CPU_DEBUG
				uart_send("TYA\r\n");
				#endif
				reg_a = reg_y;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a&0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0x40:	//rti
				if (cur_instr_index < 6)
				{
					cur_instr_index++;
				}
				switch (cur_instr_index)
				{
					case 2:
					cur_instr[1] = read_memory(0x100 + ++reg_s);
					break;
					case 3:
					cur_instr[2] = read_memory(0x100 + ++reg_s);
					break;
					case 4:
					cur_instr[3] = read_memory(0x100 + ++reg_s);
					break;
					case 6:
					#ifdef CPU_DEBUG
					uart_send("RTI\n\r");
					#endif
					reg_p = (cur_instr[1] | STATUS_EXPAND) & ~STATUS_BREAK;
					pc = cur_instr[2] | (cur_instr[3]<<8);
					cur_instr_index = 0;
					break;
					default:
					break;
				}
				break;
			case 0x60:	//rts
				if (cur_instr_index < 6)
				{
					cur_instr_index++;
				}
				switch (cur_instr_index)
				{
					case 2:
						cur_instr[1] = read_memory(0x100 + ++reg_s);
						break;
					case 3:
						cur_instr[2] = read_memory(0x100 + ++reg_s);
						break;
					case 6: 
						#ifdef CPU_DEBUG
						uart_send("RTS\n\r");
						#endif
						pc = cur_instr[1] | (cur_instr[2]<<8);
						pc++;
						cur_instr_index = 0;
						break;
					default:
						break;
				}
				break;
			case 0x20:	//jsr jump to location, save return address
				if (cur_instr_index < 3)
				{
					#ifdef CPU_DEBUG
					uart_send("JSR ");
					#endif
					fetch_opcode();
				}
				if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					write_memory(0x100+ reg_s--, (pc-1)>>8);
					cur_instr_index++;
				}
				else if (cur_instr_index == 5)
				{
					write_memory(0x100+ reg_s--, (pc-1)&0xFF);
					cur_instr_index++;
				}
				else if (cur_instr_index == 6)
				{
					uint16_t newpc = cur_instr[1] | cur_instr[2]<<8;
					#ifdef CPU_DEBUG
					uart_send("Subroutine jmp to ");
					uart_printhex(newpc);
					uart_send("\r\n");
					#endif
					pc = newpc;
					cur_instr_index = 0;
				}
				break;
			case 0x6C:	//jmp indirect
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					cur_instr[4] = read_memory((0xFF&(1+cur_instr[1])) + (cur_instr[2]<<8));
					#ifdef CPU_DEBUG
					uart_send("JMP ");
					uart_printhex(cur_instr[3] | (cur_instr[4]<<8));
					uart_send("\r\n");
					#endif
					pc = cur_instr[3] | (cur_instr[4]<<8);
					cur_instr_index = 0;
				}
				break;
			case 0x68:	//pull from stack to a
				if (cur_instr_index < 4)
				{
					cur_instr_index++;
				}
				if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("PLA\r\n");
					#endif
					reg_a = read_memory(0x100 + ++reg_s);
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0x28:	//pull from stack to p
				if (cur_instr_index < 4)
				{
					cur_instr_index++;
				}
				if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("PLP\r\n");
					#endif
					reg_p = (read_memory(0x100 + ++reg_s) | STATUS_EXPAND) & ~STATUS_BREAK;
					cur_instr_index = 0;
				}
				break;
			case 0x08:	//push flags to stack
				if (cur_instr_index < 3)
				{
					cur_instr_index++;
				}
				if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("PHP\r\n");
					#endif
					write_memory(0x100 + reg_s--, (reg_p | STATUS_EXPAND | STATUS_BREAK));
					cur_instr_index = 0;
				}
				break;
			case 0x48:	//push a to stack
				if (cur_instr_index < 3)
				{
					cur_instr_index++;
				}
				if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("PHA\r\n");
					#endif
					write_memory(0x100 + reg_s--, reg_a);
					cur_instr_index = 0;
				}
				break;
			case 0x18:	//clear carry flag
				#ifdef CPU_DEBUG
				uart_send("CLC Clear carry flag\r\n");
				#endif
				reg_p &= ~STATUS_CARRY;
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0xD8:	//clear decimal flag
				#ifdef CPU_DEBUG
				uart_send("CLD Clear decimal flag\r\n");
				#endif
				reg_p &= ~STATUS_DECIMAL;
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0xB8:	//clear overflow flag
				#ifdef CPU_DEBUG
				uart_send("CLV Clear overflow flag\r\n");
				#endif
				reg_p &= ~STATUS_OVERFLOW;
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0x38:	//set carry flag
				#ifdef CPU_DEBUG
				uart_send("SEC Set carry flag\r\n");
				#endif
				reg_p |= STATUS_CARRY;
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0xF8:	//set decimal mode
				#ifdef CPU_DEBUG
				uart_send("SED Set decimal flag\r\n");
				#endif
				reg_p |= STATUS_DECIMAL;
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0x78:	//set interrupt disable
				#ifdef CPU_DEBUG
				uart_send("SEI Set interrupt disable flag\r\n");
				#endif
				reg_p |= STATUS_INTERRUPT_DISABLE;
				read_memory(pc);
				cur_instr_index = 0;
				break;
			case 0x0A:	//shift left one bit
				#ifdef CPU_DEBUG
				uart_send("ASL A shift left one bit\r\n");
				#endif
				reg_p = (reg_p & ~STATUS_CARRY) | ((reg_a & 0x80)?STATUS_CARRY:0);
				reg_a <<= 1;
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a & 0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
				cur_instr_index = 0;
				break;
			case 0x06:	//shift left one bit zero page
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(cur_instr[1]);
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("ASL shift left one bit\r\n");
					#endif
					reg_p = (reg_p & ~STATUS_CARRY) | ((cur_instr[2] & 0x80)?STATUS_CARRY:0);
					cur_instr[2] <<= 1;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(cur_instr[1], cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x16:	//shift left one bit zero page x
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[2] = read_memory(0xFF&(reg_x+cur_instr[1]));
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("ASL shift left one bit\r\n");
					#endif
					reg_p = (reg_p & ~STATUS_CARRY) | ((cur_instr[2] & 0x80)?STATUS_CARRY:0);
					cur_instr[2] <<= 1;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(0xFF&(reg_x+cur_instr[1]), cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x0E:	//shift left absolute
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("ASL shift left one bit\r\n");
					#endif
					reg_p = (reg_p & ~STATUS_CARRY) | ((cur_instr[3] & 0x80)?STATUS_CARRY:0);
					cur_instr[3] <<= 1;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] | (cur_instr[2]<<8), cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x1E:	//shift left absolute x
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x);
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 5)
				{
					#ifdef CPU_DEBUG
					uart_send("ASL shift left one bit\r\n");
					#endif
					reg_p = (reg_p & ~STATUS_CARRY) | ((cur_instr[3] & 0x80)?STATUS_CARRY:0);
					cur_instr[3] <<= 1;
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x, cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x4A:	//shift right one bit
				#ifdef CPU_DEBUG
				uart_send("LSR A shift right one bit\r\n");
				#endif
				reg_p = (reg_p & ~STATUS_CARRY) | ((reg_a & 1)?STATUS_CARRY:0);
				reg_p = (reg_p & ~STATUS_NEGATIVE);
				reg_a >>= 1;
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
				cur_instr_index = 0;
				break;
			case 0x46:	//shift right one bit zero page
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(cur_instr[1]);
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("LSR shift right one bit\r\n");
					#endif
					reg_p = (reg_p & ~STATUS_CARRY) | ((cur_instr[2] & 1)?STATUS_CARRY:0);
					reg_p = (reg_p & ~STATUS_NEGATIVE);
					cur_instr[2] >>= 1;
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(cur_instr[1], cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x56:	//shift right one bit zero page x
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[2] = read_memory(0xFF&(reg_x+cur_instr[1]));
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("LSR shift right one bit\r\n");
					#endif
					reg_p = (reg_p & ~STATUS_CARRY) | ((cur_instr[2] & 1)?STATUS_CARRY:0);
					reg_p = (reg_p & ~STATUS_NEGATIVE);
					cur_instr[2] >>= 1;
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(0xFF&(reg_x+cur_instr[1]), cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x4E:	//shift right one bit absolute
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("LSR shift right one bit\r\n");
					#endif
					reg_p = (reg_p & ~STATUS_CARRY) | ((cur_instr[3] & 1)?STATUS_CARRY:0);
					reg_p = (reg_p & ~STATUS_NEGATIVE);
					cur_instr[3] >>= 1;
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] | (cur_instr[2]<<8), cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x5E:	//shift right one bit absolute x
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x);
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 5)
				{
					#ifdef CPU_DEBUG
					uart_send("LSR shift right one bit\r\n");
					#endif
					reg_p = (reg_p & ~STATUS_CARRY) | ((cur_instr[3] & 1)?STATUS_CARRY:0);
					reg_p = (reg_p & ~STATUS_NEGATIVE);
					cur_instr[3] >>= 1;
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x, cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x2A:	//rol
				#ifdef CPU_DEBUG
				uart_send("ROL A\r\n");
				#endif
				if (reg_a & 0x80)
				{
					reg_a <<= 1;
					reg_a |= ((reg_p & STATUS_CARRY)?1:0);
					reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
				}
				else
				{
					reg_a <<= 1;
					reg_a |= ((reg_p & STATUS_CARRY)?1:0);
					reg_p = (reg_p & ~STATUS_CARRY);
				}
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a & 0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
				cur_instr_index = 0;
				break;
			case 0x26:	//rotate left one bit zero page
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(cur_instr[1]);
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("ROL rotate left one bit\r\n");
					#endif
					if (cur_instr[2] & 0x80)
					{
						cur_instr[2] <<= 1;
						cur_instr[2] |= ((reg_p & STATUS_CARRY)?1:0);
						reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
					}
					else
					{
						cur_instr[2] <<= 1;
						cur_instr[2] |= ((reg_p & STATUS_CARRY)?1:0);
						reg_p = (reg_p & ~STATUS_CARRY);
					}
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(cur_instr[1], cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x36:	//rotate left one bit zero page x
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(0xFF&(reg_x+cur_instr[1]));
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("ROL rotate left one bit\r\n");
					#endif
					if (cur_instr[2] & 0x80)
					{
						cur_instr[2] <<= 1;
						cur_instr[2] |= ((reg_p & STATUS_CARRY)?1:0);
						reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
					}
					else
					{
						cur_instr[2] <<= 1;
						cur_instr[2] |= ((reg_p & STATUS_CARRY)?1:0);
						reg_p = (reg_p & ~STATUS_CARRY);
					}
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(0xFF&(reg_x+cur_instr[1]), cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x2E:	//rotate left one bit absolute
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] + (cur_instr[2]<<8));
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("ROL rotate left one bit\r\n");
					#endif
					if (cur_instr[3] & 0x80)
					{
						cur_instr[3] <<= 1;
						cur_instr[3] |= ((reg_p & STATUS_CARRY)?1:0);
						reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
					}
					else
					{
						cur_instr[3] <<= 1;
						cur_instr[3] |= ((reg_p & STATUS_CARRY)?1:0);
						reg_p = (reg_p & ~STATUS_CARRY);
					}
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] + (cur_instr[2]<<8), cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x3E:	//rotate left one bit absolute x
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x);
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 5)
				{
					#ifdef CPU_DEBUG
					uart_send("ROL rotate left one bit\r\n");
					#endif
					if (cur_instr[3] & 0x80)
					{
						cur_instr[3] <<= 1;
						cur_instr[3] |= ((reg_p & STATUS_CARRY)?1:0);
						reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
					}
					else
					{
						cur_instr[3] <<= 1;
						cur_instr[3] |= ((reg_p & STATUS_CARRY)?1:0);
						reg_p = (reg_p & ~STATUS_CARRY);
					}
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x, cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x6a:	//ror
				#ifdef CPU_DEBUG
				uart_send("ROR A\r\n");
				#endif
				if (reg_a & 0x1)
				{
					reg_a >>= 1;
					reg_a |= ((reg_p & STATUS_CARRY)?0x80:0);
					reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
				}
				else
				{
					reg_a >>= 1;
					reg_a |= ((reg_p & STATUS_CARRY)?0x80:0);
					reg_p = (reg_p & ~STATUS_CARRY);
				}
				reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_a & 0x80)?STATUS_NEGATIVE:0);
				reg_p = (reg_p & ~STATUS_ZERO) | ((reg_a)?0:STATUS_ZERO);
				cur_instr_index = 0;
				break;
			case 0x66:	//rotate right one bit zero page
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(cur_instr[1]);
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("ROR rotate right one bit\r\n");
					#endif
					if (cur_instr[2] & 0x1)
					{
						cur_instr[2] >>= 1;
						cur_instr[2] |= ((reg_p & STATUS_CARRY)?0x80:0);
						reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
					}
					else
					{
						cur_instr[2] >>= 1;
						cur_instr[2] |= ((reg_p & STATUS_CARRY)?0x80:0);
						reg_p = (reg_p & ~STATUS_CARRY);
					}
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(cur_instr[1], cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x76:	//rotate right one bit zero page x
				if (cur_instr_index == 1)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr[2] = read_memory(0xFF&(reg_x+cur_instr[1]));
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("ROR rotate right one bit\r\n");
					#endif
					if (cur_instr[2] & 0x1)
					{
						cur_instr[2] >>= 1;
						cur_instr[2] |= ((reg_p & STATUS_CARRY)?0x80:0);
						reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
					}
					else
					{
						cur_instr[2] >>= 1;
						cur_instr[2] |= ((reg_p & STATUS_CARRY)?0x80:0);
						reg_p = (reg_p & ~STATUS_CARRY);
					}
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[2] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[2])?0:STATUS_ZERO);
					write_memory(0xFF&(reg_x+cur_instr[1]), cur_instr[2]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x6E:	//rotate right one bit absolute
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("ROR rotate right one bit\r\n");
					#endif
					if (cur_instr[3] & 0x1)
					{
						cur_instr[3] >>= 1;
						cur_instr[3] |= ((reg_p & STATUS_CARRY)?0x80:0);
						reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
					}
					else
					{
						cur_instr[3] >>= 1;
						cur_instr[3] |= ((reg_p & STATUS_CARRY)?0x80:0);
						reg_p = (reg_p & ~STATUS_CARRY);
					}
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] | (cur_instr[2]<<8), cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x7E:	//rotate right one bit absolute x
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					cur_instr[3] = read_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x);
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 5)
				{
					#ifdef CPU_DEBUG
					uart_send("ROR rotate right one bit\r\n");
					#endif
					if (cur_instr[3] & 0x1)
					{
						cur_instr[3] >>= 1;
						cur_instr[3] |= ((reg_p & STATUS_CARRY)?0x80:0);
						reg_p = (reg_p & ~STATUS_CARRY) | STATUS_CARRY;
					}
					else
					{
						cur_instr[3] >>= 1;
						cur_instr[3] |= ((reg_p & STATUS_CARRY)?0x80:0);
						reg_p = (reg_p & ~STATUS_CARRY);
					}
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[3] & 0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[3])?0:STATUS_ZERO);
					write_memory(cur_instr[1] + (cur_instr[2]<<8) + reg_x, cur_instr[3]);
					cur_instr_index++;
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x24:
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("BIT ");
					uart_printhex(cur_instr[1]);
					uart_send("\r\n");
					#endif
					cur_instr[1] = read_memory(cur_instr[1]);
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[1]&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_OVERFLOW) | ((cur_instr[1]&0x40)?STATUS_OVERFLOW:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[1]&reg_a)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0x2C:
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("BIT ");
					uart_printhex(cur_instr[1] | (cur_instr[2]<<8));
					uart_send("\r\n");
					#endif
					cur_instr[1] = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((cur_instr[1]&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_OVERFLOW) | ((cur_instr[1]&0x40)?STATUS_OVERFLOW:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((cur_instr[1]&reg_a)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0x4C:	//jmp absolute
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 3)
				{
					uint16_t newpc = cur_instr[1] | cur_instr[2]<<8;
					#ifdef CPU_DEBUG
					uart_send("Jmp to ");
					uart_printhex(newpc);
					uart_send("\r\n");
					#endif
					pc = newpc;
					cur_instr_index = 0;
				}
				break;
			case 0x86:	//store x to memory, zero page addressing
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("STX [");
					uart_printhex(cur_instr[1]);
					uart_send("] = X\r\n");
					#endif
					write_memory(cur_instr[1], reg_x);
					cur_instr_index = 0;
				}
				break;
			case 0x96:	//store x to memory, zero page y addressing
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("STX [");
					uart_printhex(0xFF&(reg_y+cur_instr[1]));
					uart_send("] = X\r\n");
					#endif
					write_memory(0xFF&(reg_y+cur_instr[1]), reg_x);
					cur_instr_index = 0;
				}
				break;
			case 0x8E:	//store x to memory, absolute addressing
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("STX [");
					uart_printhex(cur_instr[1] | (cur_instr[2]<<8));
					uart_send("] = X\r\n");
					#endif
					write_memory(cur_instr[1] | (cur_instr[2]<<8), reg_x);
					cur_instr_index = 0;
				}
				break;
			case 0xAE:	//load x from memory, absolute addressing
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("LDX X = [");
					uart_printhex(cur_instr[1] | (cur_instr[2]<<8));
					uart_send("]\r\n");
					#endif
					reg_x = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_x&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0xBE:	//load x from memory, absolute y addressing
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("LDX X = [");
					uart_printhex(cur_instr[1] + (cur_instr[2]<<8));
					uart_send("]\r\n");
					#endif
					uint16_t ea = cur_instr[1] + (cur_instr[2]<<8) + reg_y;
					reg_x = read_memory(ea);
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_x&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x)?0:STATUS_ZERO);
					if ((ea>>8) == cur_instr[2])
					{
						cur_instr_index = 0;
					}
					else
					{
						cur_instr_index++;
					}
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0x84:	//store y to memory, zero page
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					#ifdef CPU_DEBUG
					uart_send("STY [");
					uart_printhex(cur_instr[1]);
					uart_send("] = Y\r\n");
					#endif
					write_memory(cur_instr[1], reg_y);
					cur_instr_index = 0;
				}
				break;
			case 0x94:	//store y to memory, zero page x
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("STY [");
					uart_printhex(0xFF&(cur_instr[1] + reg_x));
					uart_send("] = Y\r\n");
					#endif
					write_memory(0xFF&(cur_instr[1] + reg_x), reg_y);
					cur_instr_index = 0;
				}
				break;
			case 0x8C:	//store y to memory, absolute addressing
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("STY [");
					uart_printhex(cur_instr[1] | (cur_instr[2]<<8));
					uart_send("] = Y\r\n");
					#endif
					write_memory(cur_instr[1] | (cur_instr[2]<<8), reg_y);
					cur_instr_index = 0;
				}
				break;
			case 0xA4:	//load y from memory,zero page
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					#ifdef CPU_DEBUG
					uart_send("LDY Y = [");
					uart_printhex(cur_instr[1]);
					uart_send("]\r\n");
					#endif
					reg_y = read_memory(cur_instr[1]);
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_y&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0xB4:	//load y from memory,zero page x
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("LDY Y = [");
					uart_printhex(0xFF&(cur_instr[1] + reg_x));
					uart_send("]\r\n");
					#endif
					reg_y = read_memory(0xFF&(cur_instr[1] + reg_x));
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_y&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0xAC:	//load y from memory, absolute addressing
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 3)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 4)
				{
					#ifdef CPU_DEBUG
					uart_send("LDY Y = [");
					uart_printhex(cur_instr[1] | (cur_instr[2]<<8));
					uart_send("]\r\n");
					#endif
					reg_y = read_memory(cur_instr[1] | (cur_instr[2]<<8));
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_y&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0xBC:	//load y from memory, absolute x addressing
				if (cur_instr_index < 3)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("LDY Y = [");
					uart_printhex(cur_instr[1] + (cur_instr[2]<<8));
					uart_send("]\r\n");
					#endif
					uint16_t ea = cur_instr[1] + (cur_instr[2]<<8) + reg_x;
					reg_y = read_memory(ea);
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_y&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y)?0:STATUS_ZERO);
					if ((ea>>8) == cur_instr[2])
					{
						cur_instr_index = 0;
					}
					else
					{
						cur_instr_index++;
					}
				}
				else
				{
					cur_instr_index = 0;
				}
				break;
			case 0xA2: //ldx immediate
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 2)
				{
					#ifdef CPU_DEBUG
					uart_send("LDX X = ");
					uart_printhex(cur_instr[1]);
					uart_send("\r\n");
					#endif
					reg_x = cur_instr[1];
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_x&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0xA6:	//load x from memory,zero page
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("LDX X = [");
					uart_printhex(cur_instr[1]);
					uart_send("]\r\n");
					#endif
					reg_x = read_memory(cur_instr[1]);
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_x&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0xB6:	//load x from memory,zero page y
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				else if (cur_instr_index == 2)
				{
					cur_instr_index++;
				}
				else if (cur_instr_index == 3)
				{
					#ifdef CPU_DEBUG
					uart_send("LDX X = [");
					uart_printhex(0xFF&(cur_instr[1] + reg_y));
					uart_send("]\r\n");
					#endif
					reg_x = read_memory(0xFF&(cur_instr[1] + reg_y));
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_x&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_x)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0xA0: //ldy immediate
				if (cur_instr_index < 2)
				{
					fetch_opcode();
				}
				if (cur_instr_index == 2)
				{
					#ifdef CPU_DEBUG
					uart_send("LDY Y = ");
					uart_printhex(cur_instr[1]);
					uart_send("\r\n");
					#endif
					reg_y = cur_instr[1];
					reg_p = (reg_p & ~STATUS_NEGATIVE) | ((reg_y&0x80)?STATUS_NEGATIVE:0);
					reg_p = (reg_p & ~STATUS_ZERO) | ((reg_y)?0:STATUS_ZERO);
					cur_instr_index = 0;
				}
				break;
			case 0x50:	//branch if not overflow
				branch_flag(STATUS_OVERFLOW, 0);
				break;
			case 0x70:	//branch if overflow
				branch_flag(STATUS_OVERFLOW, STATUS_OVERFLOW);
				break;
			case 0x90:	//branch if carry clear
				branch_flag(STATUS_CARRY, 0);
				break;
			case 0x10:	//branch if not negative
				branch_flag(STATUS_NEGATIVE, 0);
				break;
			case 0x30:	//branch if negative
				branch_flag(STATUS_NEGATIVE, STATUS_NEGATIVE);
				break;
			case 0xB0:	//branch if carry set
				branch_flag(STATUS_CARRY, STATUS_CARRY);
				break;
			case 0xD0:	//branch if not zero
				branch_flag(STATUS_ZERO, 0);
				break;
			case 0xF0:	//branch if zero
				branch_flag(STATUS_ZERO, STATUS_ZERO);
				break;
			case 0xEA:	//nop
				read_memory(pc);
				#ifdef CPU_DEBUG
				uart_send("NOP\r\n");
				#endif
				cur_instr_index = 0;
				break;
			default:
				#ifdef CPU_DEBUG
				uart_send("Invalid opcode - ");
				uart_printhex(cur_instr[0]);
				uart_send(" - hang machine\r\n");
				#endif
				hang = 1;
				break;
		}
	}
}

uint8_t p6502_run_cycle()
{
	if (hang == 0)
	{
		#ifdef CPU_DEBUG
		uart_print(cycle_number++);
		uart_send(" ");
		#endif
		if (cur_instr_index < 1)
		{	//fetch opcode
			fetch_opcode();
		}
		else
		{
			decode_opcode();
		}
	}
	#ifdef CPU_DEBUG
	if ((cur_instr_index == 0) && (hang == 0))
	{
		uart_send("        A:");
		uart_printhex(reg_a);
		uart_send(" X:");
		uart_printhex(reg_x);
		uart_send(" Y:");
		uart_printhex(reg_y);
		uart_send(" P:");
		uart_printhex(reg_p);
		uart_send(" S:");
		uart_printhex(reg_s);
		uart_send(" (");
		uart_print(cycle_number);
		uart_send(")\r\n");
	}
	if ((cur_instr_index == 0) && (p6502_single_step == 1))
	{
		hang = 1;
	}
	#endif
	return hang;
}

void p6502_step()
{
	p6502_single_step = 1;
	hang = 0;
}

void p6502_resume()
{
	p6502_single_step = 0;
	hang = 0;
}

void p6502_break()
{
	hang = 1;
	p6502_single_step = 1;
}

void p6502_power_on()
{
	cur_instr_index = 0;
	cycle_number = 0;
	hang = 0;
	reg_p = 0x24;
	reg_a = 0;
	reg_x = 0;
	reg_y = 0;
	reg_s = 0xFD;
	p6502_execute_vector(RESET_VECTOR);
}