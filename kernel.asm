format MS64 COFF
public liquid_step_asm
public vector_search_asm
public multi_head_liquid_step_asm

section '.text' code readable executable

; [liquid_step_asm original conservado]
liquid_step_asm:
    push    rbp
    mov     rbp, rsp
    push    rbx
    push    rsi
    push    rdi
    mov     r10d, dword [rbp + 56]
    test    r10d, r10d
    jle     .done_legacy
    vmovaps xmm8, dqword [abs_mask]     
    vmovss  xmm6, dword [decay_factor]  
    vmovss  xmm7, dword [one_float]     
    xor     r11, r11                    
.loop_legacy:
    vxorps  xmm0, xmm0, xmm0            
    vxorps  xmm1, xmm1, xmm1            
    vxorps  xmm2, xmm2, xmm2            
    xor     rbx, rbx                    
    mov     rax, r11
    imul    rax, r10                    
.loop_k_leg:
    mov     rsi, rax
    add     rsi, rbx                    
    vmovss  xmm4, dword [rdx + rbx*4]   
    vmovss  xmm5, dword [r8 + rsi*4]
    vmulss  xmm5, xmm5, xmm4
    vaddss  xmm0, xmm0, xmm5
    vmovss  xmm5, dword [r9 + rsi*4]
    vmulss  xmm5, xmm5, xmm4
    vaddss  xmm1, xmm1, xmm5
    mov     rdi, [rbp + 48]
    vmovss  xmm5, dword [rdi + rsi*4]
    vmulss  xmm5, xmm5, xmm4
    vaddss  xmm2, xmm2, xmm5
    inc     rbx
    cmp     ebx, r10d
    jl      .loop_k_leg
    vmulss  xmm3, xmm0, xmm1            
    vandps  xmm4, xmm3, xmm8
    vaddss  xmm4, xmm4, xmm7
    vdivss  xmm3, xmm3, xmm4            
    vmulss  xmm3, xmm3, xmm2            
    vmovss  xmm4, dword [rcx + r11*4]   
    vmulss  xmm4, xmm4, xmm6            
    vaddss  xmm4, xmm4, xmm3            
    vmovss  dword [rcx + r11*4], xmm4
    inc     r11
    cmp     r11d, r10d
    jl      .loop_legacy
.done_legacy:
    pop     rdi
    pop     rsi
    pop     rbx
    pop     rbp
    ret


; void multi_head_liquid_step_asm(float* states, const float* inputs, const float* w_q, const float* w_k, const float* w_v, int count, int num_heads);
multi_head_liquid_step_asm:
    push    rbp
    mov     rbp, rsp
    sub     rsp, 64                     ; Espacio de pila local seguro para variables
    push    rbx
    push    rsi                         ; 🛡️ Protegido (Caliee-saved)
    push    rdi                         ; 🛡️ Protegido (Caliee-saved)
    push    r12
    push    r13
    push    r14
    push    r15

    ; Almacenar argumentos en variables locales de pila (inmunes a colisiones)
    mov     [rbp - 8], rcx              ; states
    mov     [rbp - 16], rdx             ; inputs
    mov     [rbp - 24], r8              ; w_q
    mov     [rbp - 32], r9              ; w_k
    mov     rax, [rbp + 48]
    mov     [rbp - 40], rax             ; w_v

    mov     r10d, dword [rbp + 56]      ; count
    mov     [rbp - 44], r10d
    
    mov     r11d, dword [rbp + 64]      ; num_heads
    mov     [rbp - 48], r11d

    test    r10d, r10d
    jle     .done_mh

    ; head_dim = count / num_heads
    mov     eax, r10d
    xor     edx, edx
    div     r11d                        
    mov     [rbp - 52], eax             ; head_dim

    vmovaps xmm8, dqword [abs_mask]
    vmovss  xmm6, dword [decay_factor]  ; 0.9
    vmovss  xmm7, dword [one_float]     ; 1.0

    mov     dword [rbp - 56], 0         ; h = 0 (cabezal actual)

.head_loop:
    mov     eax, [rbp - 56]
    cmp     eax, [rbp - 48]             ; h < num_heads?
    jge     .done_mh

    mov     dword [rbp - 60], 0         ; d = 0 (dimensión actual en el cabezal)

.dim_loop:
    mov     eax, [rbp - 60]
    cmp     eax, [rbp - 52]             ; d < head_dim?
    jge     .next_head

    ; idx = (h * head_dim) + d
    mov     eax, [rbp - 56]
    imul    eax, [rbp - 52]
    add     eax, [rbp - 60]             
    mov     edi, eax                    ; edi = idx (neurona global)

    vxorps  xmm0, xmm0, xmm0            ; Q
    vxorps  xmm1, xmm1, xmm1            ; K
    vxorps  xmm2, xmm2, xmm2            ; V

    xor     ebx, ebx                    ; k = 0
.proj_loop:
    mov     r10d, [rbp - 44]            ; count
    cmp     ebx, r10d
    jge     .proj_done

    ; Cargar input[k]
    mov     rax, [rbp - 16]             
    vmovss  xmm4, dword [rax + rbx*4]

    ; offset de matriz = idx * count + k
    mov     rax, rdi
    imul    rax, r10                    
    add     rax, rbx                    

    ; Q = input * w_q
    mov     rcx, [rbp - 24]             
    vmovss  xmm5, dword [rcx + rax*4]
    vmulss  xmm5, xmm5, xmm4
    vaddss  xmm0, xmm0, xmm5

    ; K = input * w_k
    mov     rcx, [rbp - 32]             
    vmovss  xmm5, dword [rcx + rax*4]
    vmulss  xmm5, xmm5, xmm4
    vaddss  xmm1, xmm1, xmm5

    ; V = input * w_v
    mov     rcx, [rbp - 40]             
    vmovss  xmm5, dword [rcx + rax*4]
    vmulss  xmm5, xmm5, xmm4
    vaddss  xmm2, xmm2, xmm5

    inc     ebx
    jmp     .proj_loop

.proj_done:
    vmulss  xmm3, xmm0, xmm1
    vandps  xmm4, xmm3, xmm8
    vaddss  xmm4, xmm4, xmm7
    vdivss  xmm3, xmm3, xmm4
    vmulss  xmm3, xmm3, xmm2

    ; states[idx] = (states[idx] * decay) + attention
    mov     rcx, [rbp - 8]              
    movsxd  rax, edi
    vmovss  xmm4, dword [rcx + rax*4]
    vmulss  xmm4, xmm4, xmm6
    vaddss  xmm4, xmm4, xmm3
    vmovss  dword [rcx + rax*4], xmm4

    inc     dword [rbp - 60]            ; d++
    jmp     .dim_loop

.next_head:
    inc     dword [rbp - 56]            ; h++
    jmp     .head_loop

.done_mh:
    pop     r15
    pop     r14
    pop     r13
    pop     r12
    pop     rdi                         ; 🛡️ Restaurado
    pop     rsi                         ; 🛡️ Restaurado
    pop     rbx
    mov     rsp, rbp
    pop     rbp
    ret


; int vector_search_asm(const float* query, const float* database, int vector_count, int dim);
vector_search_asm:
    push    rbp
    mov     rbp, rsp
    push    rbx
    push    rsi
    push    rdi
    push    r12
    push    r13
    push    r14
    push    r15
    mov     r10, -1                 
    mov     eax, 0xFF800000         
    vmovd   xmm15, eax              
    xor     r11, r11                
.vector_loop:
    cmp     r11d, r8d
    jge     .done_search
    mov     rax, r11
    imul    rax, r9
    shl     rax, 2                  
    lea     rsi, [rdx + rax]        
    vxorps  ymm0, ymm0, ymm0        
    xor     rbx, rbx                
.dim_loop_vs:
    cmp     ebx, r9d
    jge     .dim_done_vs
    vmovups ymm1, [rcx + rbx*4]
    vmovups ymm2, [rsi + rbx*4]
    vfmadd231ps ymm0, ymm1, ymm2
    add     ebx, 8                  
    jmp     .dim_loop_vs
.dim_done_vs:
    vextractf128 xmm1, ymm0, 1      
    vaddps  xmm0, xmm0, xmm1        
    vmovhlps xmm1, xmm0, xmm0       
    vaddps  xmm0, xmm0, xmm1        
    vmovaps xmm1, xmm0
    shufps  xmm1, xmm1, 1
    vaddss  xmm0, xmm0, xmm1        
    ucomiss xmm0, xmm15
    jbe     .next_vector
    vmovaps xmm15, xmm0             
    mov     r10, r11                
.next_vector:
    inc     r11
    jmp     .vector_loop
.done_search:
    mov     rax, r10                
    pop     r15
    pop     r14
    pop     r13
    pop     r12
    pop     rdi
    pop     rsi
    pop     rbx
    pop     rbp
    ret

section '.data' data readable writeable align 16
    abs_mask     dd 0x7FFFFFFF, 0x7FFFFFFF, 0x7FFFFFFF, 0x7FFFFFFF
    one_float    dd 1.0
    decay_factor dd 0.9