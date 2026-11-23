# ⚛️ Local Collider (`local_collider`)

Um simulador de **acelerador de partículas em CLI** escrito em **Rust** que converte **processamento real de CPU** (Hashes SHA-256 + transformações matriciais de Lorentz em ponto flutuante / GFLOPS) em velocidade relativística ($\beta = v/c$, fator de Lorentz $\gamma$, energia em $\text{TeV}$) e **probabilidade estatística real** (distribuições de Poisson, Gaussiana, ressonância de Breit-Wigner e significância de $5\sigma$) de descobrir partículas e anomalias quânticas.

O projeto foi desenhado em **dois processos independentes** que se comunicam via TCP IPC (`127.0.0.1:7878`):
1. **`collider-engine`** *(Daemon / Motor em CLI)*: Gerencia as threads reais de processamento da CPU, mede telemetria do sistema (`sysinfo`), calcula a cinemática relativística, rola as colisões estatísticas alimentadas pela entropia real da CPU e salva descobertas em disco.
2. **`collider-viz`** *(Animação em CLI à parte)*: Conecta-se ao motor em execução a partir de outro terminal e renderiza uma **animação TUI em tempo real** (via `ratatui` + Braille Canvas) do anel síncrotron, pacotes de prótons, jatos do calorímetro central e painel de controle remoto.

---

## 🏗️ Arquitetura de Dois Processos

```text
┌─────────────────────────────────────────┐         TCP JSON IPC         ┌─────────────────────────────────────────┐
│  Terminal 1: collider-engine (Daemon)   │ ◄──────────────────────────► │   Terminal 2: collider-viz (TUI Anim)   │
│  • Pool de threads CPU (SHA-256 + FMA)  │        127.0.0.1:7878        │  • Anel Síncrotron em Canvas Braille    │
│  • Cinemática Relativística (β, γ, TeV) │                              │  • Jatos curvos de colisões no detector │
│  • Colisões de Poisson & Breit-Wigner   │ ◄──────────────────────────► │  • Controle remoto de Threads/Modo/RF   │
│  • Persistência (collider_state.json)   │   Terminal 3: CLI `status`   │  • Códice Quântico 5σ em tempo real     │
└─────────────────────────────────────────┘         e `tune`             └─────────────────────────────────────────┘
```

---

## 🚀 Como Executar

### 1. Inicie o Motor do Acelerador (Processo 1)

No primeiro terminal, inicie o daemon que vai trocar ciclos do seu processador por aceleração de feixe:

```bash
cargo run --release --bin collider-engine -- --threads 4 --intensity 80 --mode synchrotron
```

*(Ou usando o binário unificado: `cargo run --release -- engine --threads 4 --intensity 80`)*

### 2. Abra a Animação em CLI à Parte (Processo 2)

Em **outro terminal**, abra o visualizador TUI animado que se conecta automaticamente ao motor:

```bash
cargo run --release --bin collider-viz
```

*(Ou usando o binário unificado: `cargo run --release -- viz`)*

### 3. (Opcional) Controle ou Consulte via CLI (Terminal 3)

Você também pode consultar a telemetria, inspecionar o Códice Quântico ou ajustar como o feixe acelera via linha de comando:

```bash
# Ver telemetria instantânea do motor
cargo run --release -- status

# Alterar modo de aceleração, sintonizar harmônico RF e disparar pulso supercrítico
cargo run --release -- tune --mode quantum --harmonic-delta 0.2 --burst

# Listar o Códice completo com as 12 partículas e anomalias quânticas
cargo run --release -- codex
```

---

## ⚙️ Mecânica: "Quanto Mais Acelera e Como Acelera"

1. **Quanto Mais Acelera (CPU $\to$ Velocidade Relativística)**:
   - Cada thread ativa executa rodadas reais de **SHA-256** combinadas com **álgebra de quadrivetores de Lorentz ($4\times 4$ FMA)**.
   - A taxa medida em **kH/s** e **GFLOPS** bombeia momento para o anel acumulador, elevando a energia do feixe ($E$ em $\text{TeV}$), o fator de Lorentz ($\gamma = 1 + E / m_p c^2$) e aproximando a velocidade $\beta = v/c = \sqrt{1 - 1/\gamma^2}$ de $1c$ (medida em **"noves de $c$"**, ex: $0.999999999c$).
   - A entropia criptográfica dos hashes calculados pela sua CPU alimenta diretamente o gerador estatístico das colisões.

2. **Como Acelera (4 Modos + Sintonia RF + Foco Quadrupolar + Capacitor)**:
   - **`Linac RF Estável` (`linac`)**: $98\%$ de estabilidade de fase, ideal para calibração inicial e léptons/mésons leves.
   - **`Síncrotron Harmônico` (`synchrotron`)**: Equilíbrio clássico de anel com alta luminosidade ($\times 1.65$) e colisões constantes.
   - **`Plasma Wakefield` (`plasma`)**: Surfe em onda de plasma a laser; multiplica a energia de pico ($\text{TeV}$) em $\times 2.4$, ao custo de menor estabilidade.
   - **`Ressonância de Vácuo Quântico` (`quantum`)**: Modulação harmônica voltada para tunelamento sub-limiar e anomalias da escala de Planck ($\times 2.85$ chance quântica).
   - **Sintonia de Harmônico RF ($\text{GHz}$)**: Cada partícula/anomalia possui uma frequência ressonante regida por uma curva Lorentziana de **Breit-Wigner**. Sintonizar o harmônico RF perto do alvo multiplica a seção de choque e a energia!
   - **Foco Magnético Quadrupolar ($\text{T/m}$)**: Comprime a seção transversal do feixe, aumentando a luminosidade integrada ($\text{fb}^{-1}$) e as colisões por segundo (distribuição de **Poisson**).

---

## 🌌 Códice Quântico (12 Tiers de Descoberta $5\sigma$)

| Tier | Símbolo | Partícula / Fenômeno Quântico | Limiar ($\text{TeV}$) | Harmônico RF | Categoria |
|:---:|:---:|---|---:|---:|---|
| 1 | `μ⁺μ⁻` | Par de Múons Relativísticos | `0.12 TeV` | `1.0 GHz` | Modelo Padrão (Léptons) |
| 1 | `J/ψ` | Méson J/ψ (Charmônio) | `0.45 TeV` | `1.3 GHz` | Modelo Padrão (Hádrons) |
| 2 | `Z⁰` | Bóson Vetorial Z⁰ | `1.25 TeV` | `1.7 GHz` | Bóson de Calibre Eletrofraco |
| 2 | `H⁰` | Bóson de Higgs Escalar | `2.90 TeV` | `2.1 GHz` | Quebra Espontânea de Simetria |
| 3 | `Tcc⁺` | Tetraquark Duplo-Charmoso | `6.20 TeV` | `2.4 GHz` | Hádron Exótico Multiquark |
| 3 | `Pc(4457)⁺` | Pentaquark de Valência Charm | `11.50 TeV` | `2.8 GHz` | Hádron Exótico Multiquark |
| 4 | `χ̃₁⁰` | Neutralino Supersimétrico | `21.00 TeV` | `3.2 GHz` | Supersimetria (Matéria Escura) |
| 4 | `LQ₃` | Leptoquark Vetorial de 3ª Geração | `38.00 TeV` | `3.6 GHz` | Grande Unificação (Além do MP) |
| 5 | `G_KK` | Gráviton de Kaluza-Klein | `68.00 TeV` | `4.0 GHz` | Gravidade Quântica / Dimensões Extras |
| 5 | `g_D` | Monopolo Magnético de Dirac | `115.00 TeV` | `4.3 GHz` | Topologia Quântica de Gauge |
| 6 | `Ψ_foam` | Condensado de Espuma Quântica | `185.00 TeV` | `4.7 GHz` | Anomalia da Escala de Planck |
| 6 | `Ω_brane` | Singularidade de Tunelamento de Brana | `300.00 TeV` | `4.9 GHz` | Singularidade Quântica Profunda |

---

## ⌨️ Atalhos do Visualizador TUI (`collider-viz`)

- **`[M]`**: Alternar modo de aceleração (*Linac* $\to$ *Síncrotron* $\to$ *Plasma Wakefield* $\to$ *Ressonância Quântica*)
- **`[+]` / `[-]`**: Aumentar / diminuir número de threads de CPU no motor
- **`[]]` / `[[]`**: Aumentar / diminuir intensidade de uso de CPU ($10\% \dots 100\%$)
- **`[↑]` / `[↓]`**: Sintonizar frequência do Harmônico RF ($\text{GHz}$) para casar com a ressonância de Breit-Wigner do próximo alvo
- **`[←]` / `[→]`**: Ajustar foco magnético dos ímãs quadrupolos ($\text{T/m}$)
- **`[ESPAÇO]` ou `[B]`**: Disparar Pulso Supercrítico do capacitor (sobrecarga de $3.25\times$ em $\text{TeV}$ e seção de choque quântica)
- **`[R]`**: Reiniciar estado salvo do colisor
- **`[Q]` ou `[ESC]`**: Fechar o visualizador TUI (o motor `collider-engine` continua rodando em background)

---

## 📄 Licença

Distribuído sob a licença **MIT**. Veja [LICENSE](LICENSE) para mais informações.
