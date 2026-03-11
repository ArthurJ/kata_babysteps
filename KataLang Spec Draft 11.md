## Especificação do Modelo de Execução e Memória

Este documento estabelece as regras de tempo de execução (runtime), o modelo de concorrência e a topologia de memória da Kata-Lang. O design é otimizado para concorrência massiva de I/O sem penalizar o processamento matemático puro.

### 1\. Paradigma de Avaliação (Eager Evaluation)

A Kata-Lang utiliza avaliação **Estrita (Eager)** em tempo de execução.

Os argumentos de uma função são avaliados integralmente antes de a execução entrar no corpo da função. O conceito de "Grafo Acíclico Dirigido (DAG)" referido na especificação não existe como estrutura de dados em runtime (não há alocação de *Thunks*). O DAG é estritamente uma Representação Intermediária (IR) em tempo de compilação, utilizada para aplicar *Stream Fusion* (fundir loops adjacentes) e *Constant Folding*.

### 2\. Modelo de Concorrência (M:N via Tokio)

A linguagem suporta concorrência assíncrona de uso geral através de um escalonador cooperativo M:N (*Green Threads*), integrado no runtime embutido.

#### 2.1 Separação Arquitetural de Compilação

A compilação do código divide-se em dois domínios gerados pelo backend (Cranelift):

* **Functions (Processamento de CPU):** Sendo puras e desprovidas de I/O, as funções são geradas como código de máquina linear síncrono. Elas operam na pilha (stack) nativa da *thread* do Sistema Operativo. O escalonador não as interrompe no meio da sua execução lógica.  
* **Actions (I/O e Efeitos):** Sendo o domínio impuro, o corpo de uma `Action` é compilado como uma Máquina de Estado (uma estrutura que encapsula o estado das variáveis locais). Sempre que uma `Action` invoca um canal ou uma operação de sistema bloqueante, a máquina de estado cede o controlo (`yield`) ao escalonador, que aloca a CPU para outra *Green Thread*.

Esta separação garante que o custo de abstração do I/O assíncrono não contamina a performance de algoritmos matemáticos ou de transformação de dados.

### 3\. Topologia de Memória, Ciclo de Vida e Ownership

A Kata-Lang não possui *Garbage Collector* tradicional de varredura (Tracing GC), nem exige um *Borrow Checker* restritivo. A gestão de posse (Ownership) e de tempo de vida é resolvida organicamente pelas regras de imutabilidade de `Data` e pelo escopo das Arenas.

#### 3.1 Ownership Local (Arenas Zero-Cost)

* **Propriedade:** Cada `Action` instanciada é a proprietária exclusiva de uma *Arena* (um *bump allocator* de bloco de memória contígua).  
* **Ausência de Aliasing Mutável:** Como os dados são estritamente imutáveis, o compilador autoriza múltiplas referências (aliasing) para o mesmo endereço de memória dentro da Arena sem risco de concorrência. A palavra-chave `var` na Action apenas muta a referência na *Stack*, nunca o dado na Arena.  
* **Ciclo de Vida:** Todas as alocações locais são afixadas na Arena e inicializadas com um cabeçalho de metadados reservado para promoções. Não há limpeza individual de objetos. Quando a `Action` conclui, a Arena inteira é libertada em tempo `O(1)`, garantindo que nenhuma referência sobrevive ao dono original.

#### 3.2 Ownership Partilhado e Promoção (Global Heap)

A memória só sobrevive à destruição da Arena local se sofrer "Escape" através da passagem de mensagens inter-processos. O compilador não permite o envio de ponteiros da Arena local por canais.

Quando um dado é enviado via `channel!`, o runtime executa a seguinte rotina de promoção:

1. **Verificação de Encaminhamento:** O runtime verifica o cabeçalho do objeto na Arena.  
2. **Promoção Inédita:** Se o cabeçalho estiver vazio, o dado é copiado fisicamente para a **Heap Global** partilhada. O bloco global recebe um encapsulamento ARC (Atomic Reference Counting) iniciado em 1\. O endereço deste novo bloco global é então gravado no cabeçalho do objeto original na Arena (Forwarding Pointer).  
3. **Múltiplos Envios (Broadcast):** Se a mesma Action tentar enviar o mesmo dado para um segundo canal, o runtime deteta o Forwarding Pointer no cabeçalho. Em vez de duplicar a memória, ele segue o ponteiro até à Heap Global e realiza um incremento atómico no ARC (passando para 2, 3, etc.).  
4. **Reencaminhamento Inter-Actions:** Se uma Action receber um dado da Heap Global e o repassar para outro canal, o runtime apenas incrementa o ARC atómico do bloco global. O ARC nunca é incrementado por atribuições de variáveis locais.

Esta arquitetura isola o custo da sincronização de memória atómica estritamente ao I/O de mensagens, evita duplicações desnecessárias na memória partilhada, e resolve o ownership sem intervenção manual do programador.

### 4\. O Runtime da Kata-Lang e Geração de Binário

O binário final gerado é um executável estático autossuficiente (Standalone) que encapsula o código gerado pelo Cranelift e o pacote do Runtime.

#### 4.1 Eliminação de Código Morto (Tree-Shaking Perfeito)

A Kata-Lang proíbe a reflexão (Reflection) e a invocação dinâmica baseada em strings em tempo de execução. Esta restrição permite que o compilador realize uma eliminação de código morto matemática e determinística.

Durante a Otimização da AST, o compilador constrói um Grafo de Chamadas (Call Graph) partindo exclusivamente do ponto de entrada do programa (a `Action` principal, usualmente `main`).

* Qualquer função, bloco de dados, contrato de interface ou módulo externo que não faça parte da árvore de dependências diretas ou indiretas deste grafo é extirpado.  
* O Cranelift nunca recebe instruções para gerar IR ou Assembly de código inalcançável.  
* Isto garante que as bibliotecas padrão pesadas não penalizam o tamanho do executável, compensando o espaço ocupado pelo escalonador embutido.

#### 4.2 Responsabilidades da Camada do Runtime

O Runtime é uma camada compacta acoplada ao binário responsável por:

* Inicializar o *pool* de threads nativas do Sistema Operativo e arrancar o reator de I/O assíncrono (Tokio).  
* Gerir as estruturas atómicas dos Canais de Comunicação (CSP).  
* Administrar a Heap Global, gerir os contadores ARC e libertar os ponteiros órfãos.  
* Fornecer os adaptadores FFI (Foreign Function Interface) entre o Assembly gerado pelo Cranelift e as Syscalls do Sistema Operativo.

## Especificação Léxica e Parsing

O design prioriza a simplicidade de *tokenização*, eliminando ambiguidades sintáticas e minimizando o *lookahead* (leitura antecipada) do compilador.

### 1\. Aridade, Funções Variádicas e Aplicação Parcial

A linguagem faz uma distinção estrita no *parsing* baseada no domínio de execução (Functions vs. Actions) e impõe regras rígidas de consumo de argumentos.

#### 1.1 Funções (Aridade Fixa Estrita)

* **Regra:** Todas as `Functions` (puras) possuem aridade fixa e conhecida estaticamente. Funções variádicas são expressamente proibidas no domínio funcional.  
* **Mecânica do Parser:** Como a aridade é rígida, a notação prefixa pura (ex: `+ * 2 3 4`) é resolvida de forma determinística em tempo `O(N)`. O compilador sabe exatamente quantos argumentos empilhar na AST para cada função, dispensando delimitadores.

#### 1.2 Currying Explícito (O Operador Hole `_`)

Para realizar aplicação parcial de uma função em escopo livre (sem uso de parênteses), a linguagem exige o preenchimento da aridade através do operador *Hole* (`_`).

* **Mecânica Sintática:** O Parser avalia o `_` como um argumento estruturalmente válido, satisfazendo a contagem de aridade exigida pela notação prefixa.  
* **Mecânica Semântica:** o *Type Checker* intercepta a chamada contendo *Holes* e converte-a numa *closure* (Lambda) que aguarda a injeção dos argumentos faltantes.  
* *Sintaxe:* `let f + 1 _`  
* *AST Primária :* `Application(+, Args:[1, Hole])`  
* *AST Tipada :* `Lambda(Arity: 1, Closure: [1])`

#### 1.3 Actions (Marcador Imperativo e Variadismo)

* **Regra:** Todas as chamadas a `Actions` devem ser explicitamente sufixadas com um ponto de exclamação `!` (ex: `echo!`, `channel!`). Ao contrário das funções, as `Actions` podem ser variádicas.  
* **Mecânica do Parser:** O Lexer reconhece o `!` como o marcador imperativo. Para evitar a quebra da leitura da notação prefixa, uma chamada variádica numa `Action` exige o encapsulamento de todos os argumentos dentro de parênteses (formando uma tupla léxica única).  
  * *Válido:* `echo! ("Erro no processamento" id_usuario data)`  
  * *Inválido:* `echo! "Erro no processamento" id_usuario data` (O parser aplicaria `echo!` apenas à primeira string, tratando o resto como instruções desconexas).

### 2\. Strings e Templates (Dados Cegos)

O Lexer da Kata-Lang não realiza interpretação de código embutido dentro de blocos de texto.

* **Regra:** *Strings* (textos delimitados por aspas duplas `"..."`) são "dados cegos" absolutos.  
* **Mecânica do Lexer:** Não existe modo de interpolação léxica (como `"$var"` ou `"${expr}"`). Ao encontrar uma aspa, o Lexer consome todos os caracteres como texto plano até à aspa de fecho.  
* **Resolução:** A injeção de variáveis em textos é delegada para o domínio de execução funcional através de funções de formatação e *Templates* explícitos (ex: `format "Olá {}" (nome)`).

### 3\. A Teoria Unificada das Tuplas e Parênteses

A Kata-Lang não distingue sintaticamente entre "agrupamento matemático", "lista de parâmetros" e "estrutura de dados Tupla". O token `(...)` cria invariavelmente um nó `Tuple` na AST. A semântica é resolvida no Type Checker através das seguintes regras:

#### 3.1 Delimitadores Internos

Dentro de parênteses, os itens podem ser separados por espaços ou vírgulas. Para o parser, são tokens equivalentes. `(1 2 3)` produz a mesma AST que `(1, 2, 3)`.

#### 3.2 Avaliação Semântica da Tupla

Ao avaliar um nó `Tuple`, o *Type Checker* inspeciona o primeiro elemento (Índice 0):

1. **Aplicação de Função:** Se o Índice 0 for invocável, a tupla é transformada num nó de Aplicação (Call).  
   * *Entrada:* `(+ 1 1)` \-\> *AST:* `Application(+, Args:[1, 1])`  
2. **Redução de Unidade:** Uma tupla contendo apenas um elemento não-invocável resolve-se para o próprio elemento. Se for uma função não aplicada, resolve-se para a referência da função (Lambda).  
   * *Entrada:* `(42)` \-\> *AST:* `Literal(42)`  
   * *Entrada:* `(+)` \-\> *AST:* `Lambda(+, Arity: 2)`  
3. **Currying Implícito:** Se uma função for invocada dentro da tupla com menos argumentos que a sua aridade estrita, o compilador infere o *currying* automaticamente, dispensando o operador *Hole*. Os delimitadores `(...)` justificam o corte léxico prematuro.  
   * *Entrada:* `(+ 1)` \-\> *AST:* `Lambda(Closure:[1], Arity: 1)`  
4. **Dados Literais:** Se o Índice 0 for um dado não-invocável e a tupla tiver múltiplos elementos, ela permanece como uma estrutura `Tuple` literal.  
   * *Entrada:* `(1 "Teste")` \-\> *AST:* `Tuple([1, "Teste"])`

#### 3.3 Tuplas Implícitas em Assinaturas

Na definição de assinaturas de tipos, os delimitadores `::` e `=>` dispensam o uso de parênteses. A sequência de tipos definida entre eles é analisada diretamente como um `TupleType` único.

* *Sintaxe:* `soma :: Int Int => Int` gera estritamente a mesma AST que `soma :: (Int Int) => Int`.

### 4\. Escopo e Terminadores (Significant Whitespace)

#### 4.1 Terminador de Instrução

O caractere de quebra de linha `\n` é o terminador oficial. O Parser encerra o ramo atual da AST ao encontrar uma quebra de linha (exceto se a expressão estiver aberta dentro de um nó de fechamento pendente `(...)` ou `[...]`).

#### 4.2 Escopo por Indentação

* **Regra:** Blocos de escopo são definidos pelo nível de indentação em relação à linha de declaração.  
* **Uniformidade:** O uso de espaços ou *tabs* (`\t`) é válido, contanto que seja estritamente uniforme dentro daquele bloco específico. A mistura resulta num erro léxico abortivo.  
* **Mecânica do Lexer:** O Lexer rastreia a indentação no início de cada linha, emitindo tokens sintéticos `INDENT` (abertura de escopo) e `DEDENT` (fecho de escopo).

##   Sistema de Tipos e Interfaces

Regras do sistema de tipos da Kata-Lang, focando na separação estrita entre dados e código executável, o mecanismo de Early Checking para funções genéricas, e a manipulação puramente funcional de Tipos-Refinados.

### 1\. Separação Estrita: Tipos, Dados e Funções

No motor do compilador (TypeEnv), toda entidade manipulável possui um **Tipo** (ex: Int, List::T ou \[T\], (A \-\> B)). Funções são cidadãs de primeira classe e podem ser passadas como argumentos.

No entanto, a categoria **Dado (Data)** é um subconjunto estritamente restrito:

* **Dados (Data):** São inertes, imutáveis, livres de comportamento e **nativamente serializáveis**. Englobam primitivos, coleções persistentes, arrays, tensores e uniões (ADTs).  
* **Funções (Lambdas):** *Não* são Dados. Lambdas formam *closures* (capturam ambiente imutável e possuem ponteiros de execução de máquina). Por estarem atreladas ao *layout* de memória do binário compilado, não possuem portabilidade universal:  
  * **IPC Local (@parallel):** O compilador *permite* o envio de Lambdas entre processos locais do mesmo binário. Isto é resolvido internamente de forma opaca (via tabelas de despacho estáticas), sem necessidade de serialização real.  
  * **I/O de Rede e Persistência:** O compilador *rejeitará* a tentativa de tratar um Lambda como um "Dado" para ser exportado (ex: JSON sobre TCP). O envio de código pela rede (processamento distribuído) ou execução dinâmica (REPL) exige a serialização da Representação Intermediária (IR) e a utilização de um compilador JIT no destino, mecanismos que transcendem o contrato de um Dado inerte.

### 2\. Interfaces e Despacho Múltiplo (Multiple Dispatch)

A linguagem não possui métodos atrelados a classes. O polimorfismo *ad-hoc* ocorre através de Interfaces e implementações de Despacho Múltiplo.

#### 2.1 Declaração de Contratos (Escopo de Módulo)

Tipos de dados estruturais não sabem quais comportamentos possuem no momento da sua definição estrutural. O contrato não é aninhado na estrutura, nem é global ao programa inteiro; é firmado no **nível superior do módulo (top-level)**, de forma isolada.

Isto significa que as rotas de despacho ficam registadas no TypeEnv do ficheiro atual. Quando um tipo é exportado e importado por outro módulo, as implementações de interface atreladas a ele viajam em conjunto, garantindo a interoperabilidade sem poluir o escopo global.

\# Definição estrutural pura  
data Vec2 (x y)

\# Declaração do contrato no top-level do módulo  
Vec2 implements ADD\_BEHAVIOR  
    \# Assinaturas explícitas definem as rotas do despacho  
    \+ :: Vec2 Float \=\> Vec2  
    lambda (v f) ...

    \+ :: Vec2 Vec2 \=\> Vec2  
    lambda (v1 v2) ...

### 3\. Generics e Early Checking

A Kata-Lang adota o modelo de **Early Checking** para funções genéricas. O compilador prova matematicamente a validade de uma função genérica no momento de sua definição, sem depender de "hints" do momento em que a função for chamada (*Late Checking*).

Se o compilador não conseguir unificar os tipos internamente, ele exigirá que o programador assine um contrato explícito no bloco with.

`# Função genérica com Early Checking`  
`soma_generica :: A B => C`  
`lambda (x y) + x y`  
`with`   
    `# A restrição obriga a existência prévia desta assinatura no TypeEnv`  
    `+ :: A B => C`

### 4\. Tipos-Refinados e Smart Constructors

Tipos-Refinados aplicam restrições matemáticas e lógicas sobre um tipo base usando tuplas e o operador Hole (\_).

Estados inválidos são irrepresentáveis.

#### 4.1 Sintaxe e Construtores

data PositiveInt as (Int, \> \_ 0\)

O compilador gera automaticamente um *Smart Constructor* para o tipo-refinado.

* **Retorno do Construtor:** Sempre retorna Result::T::Err.  
* **O tipo Err:** É um alias para Text (String). A mensagem de erro não precisa de ser escrita pelo utilizador; o compilador injeta automaticamente a expressão do predicado violado (ex: "Value '-5' does not satisfy the predicate '\> \_ 0'").

#### 4.2 Degradação Matemática e Recuperação de Tipo

As operações matemáticas aplicadas a tipos-refinados atuam sobre o tipo base subjacente. A pureza matemática é mantida pela **degradação do tipo**.

Se `a` e `b` são `PositiveInt`, a operação `(- a b)` retorna estritamente um `Int`. O compilador emite um erro de tipagem caso o programa tente usar esse resultado diretamente onde um `PositiveInt` é exigido.

Para recuperar o tipo-refinado após uma operação matemática, o programador tem dois caminhos:

1. **Comportamento Padrão (Via Construtor Dinâmico):** O programador submete o resultado degradado de volta ao construtor do tipo (ex: `PositiveInt (- a b)`). O construtor fará a validação em tempo de execução e retornará, obrigatoriamente, um `Result::PositiveInt::Err`.  
2. **Sobrecarga Pura (Via Implementação Explícita):** O programador pode definir funções ou assinar contratos que interceptam o tipo subjacente e tratam a violação internamente (provendo fallbacks ou provas lógicas). Isso permite contornar a mecânica padrão do `Result`, garantindo que a função retorne o tipo-refinado original em estado puro (detalhado na Seção 5).

#### 4.3 Fallbacks Literais Estáticos

Literais numéricos constantes inseridos no código-fonte são avaliados em tempo de compilação. Se o literal passar no predicado (ex: o literal 1 num contexto que exige PositiveInt), o compilador aceita-o como um tipo nativo PositiveInt, dispensando o construtor dinâmico e o retorno de Result.

### 5\. Tratamento de Erros e Padrões Funcionais Puros

Para manter a pureza da Mônada Funcional, a construção imperativa `match` é **proibida dentro de lambdas**. Para desestruturar e reagir a um `Result` gerado por um tipo-refinado, a linguagem exige a composição com lambdas de Pattern-Matching.

#### Exemplo: Sobrecarga Pura com Resolução de Result

`Vec2 implements ADD_BEHAVIOR`  
    `# Prometemos que a saída será sempre PositiveInt (sem Result)`  
    `+ :: PositiveInt Int => PositiveInt`  
    `lambda (p i)`  
        `# 1. A soma (+ p i) degrada para Int.`  
        `# 2. O construtor 'PositiveInt' reavalia e retorna Result.`  
        `# 3. O Pipe despacha para os lambdas de resolução de padrão.`  
        `PositiveInt (+ p i) |> (`  
            `lambda (Ok valor_puro) valor_puro  	# Ramo de Sucesso`  
            `lambda (Err erro) 1 				# Ramo de Falha`  
        `) _`  
	`# O fallback deve respeitar o tipo de retorno.`  
`# '1' é aceito estaticamente pelo compilador como PositiveInt.`	

Se o programador não quiser fornecer um fallback (como no caso acima), a linguagem obriga a alterar a assinatura principal da função para propagar o Result, delegando o tratamento imperativo da falha (como emissão de panic\!) para as Actions.

