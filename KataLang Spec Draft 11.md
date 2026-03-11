## Kata-Lang Standard Library (StdLib)

Especificação das primitivas embutidas (*built-ins*) e funções da biblioteca padrão. Todas as funções listadas aqui respeitam a pureza referencial e avaliam de forma estrita, enquanto as *actions* (sufixadas com `!`) interagem com o escalonador e recursos do sistema operacional.

1\. Functions (Domínio Puro)

As funções operam sobre dados e retornam dados. Não possuem efeitos colaterais. O compilador aplica fusão de fluxo (*Stream Fusion*) sempre que funções iteráveis são encadeadas.

1.1. Operadores Matemáticos e Lógicos (Interface `NUM`, `ORD`, `EQ`)

Resolvidos por *Multiple Dispatch* em tempo de compilação.

* `+ :: NUM NUM => NUM` (Soma)  
* `- :: NUM NUM => NUM` (Subtração)  
* `* :: NUM NUM => NUM` (Multiplicação, suporta produto de tensores)  
* `/ :: NUM NonZero => Float` (Divisão exata. Requer que o divisor seja estritamente não-zero via Tipo Refinado, eliminando a necessidade de `Result`).  
* `// :: NUM NonZero => Int` (Divisão inteira. Requer Tipo Refinado).  
* `div :: NUM NUM => Result::NUM::Err` (Divisão segura em tempo de execução. Valida o divisor dinamicamente e retorna `Ok NUM` ou `Err "Divisão por zero"`).  
* `** :: NUM NUM => NUM` (Exponenciação)  
* `mod :: Int NonZero => Int` (Módulo. Requer Tipo Refinado no divisor).  
* `abs :: NUM => NUM` (Valor absoluto)  
* `sin, cos, tan :: Float => Float` (Trigonometria)  
* `sqrt :: Float => Float` (Raiz quadrada)  
* `=:: EQ EQ => Bool` (Comparações)  
* `>, <, >=, <= :: ORD ORD => Bool` (Comparações)  
* `and, or :: Bool Bool => Bool` (Operadores lógicos booleanos)

1.2. Álgebra Linear (Interface `Tensor`)

* `dot :: Tensor Tensor -> Tensor` (Produto Escalar. Requer `DOT_BEHAVIOR`).  
* `shape :: Tensor -> Tuple` (Retorna a tupla de dimensões de um tensor).  
* `scalar :: Tensor::T::() -> T` (Recebe um Tensor 0D e retorna um escalar NUM).

1.3. Manipulação de Coleções e Fluxos (`ITERABLE`)

* `map :: (A -> B) Iterable::A -> Iterable::B` (Aplica a função a cada elemento).  
* `filter :: (A -> Bool) Iterable::A -> Iterable::A` (Filtra elementos via predicado).  
* `fold :: (B A -> B) B Iterable::A -> B` (Acumulação de valores com seed inicial)  
* `reduce :: (A A -> A) Iterable::A -> Result::A::Err` (Acumulação de valores).  
* `zip :: Iterable::A Iterable::B -> Iterable::(A B)` (Agrupa iteráveis em tuplos).  
* `head :: List::T -> Optional::T` (Retorna o primeiro elemento de uma lista persistente).  
* `tail :: List::T -> Optional::List::T` (Retorna a lista sem o primeiro elemento).  
* `at :: (Coordenadas) Colecao -> Result::T::Err` (Extração posicional segura com validação de limites *Out-of-Bounds* que retorna `Result`).

1.4. Introspecção e Formatação

* `str :: SHOW -> Text` (Stringificação de qualquer tipo que implemente a interface `SHOW`).  
* `format :: Text Args... -> Text` (Template Funcional para interpolação explícita de strings).  
* `type :: T -> Text` (Retorna a representação estrita do tipo do objeto em runtime).  
* `fields :: T -> List::Text` (Retorna os campos/chaves de uma estrutura de dados em runtime).  
* `int, float :: NUM -> NUM` (Cast de precisão nativa).

2\. Actions (Domínio Impuro)

As *Actions* interagem com o *Runtime* Kata, o Sistema Operacional e orquestram a simultaneidade. Não podem ser chamadas a partir de Funções.

2.1. Concorrência e Escalonamento (Modelo CSP)

* `fork! :: Action -> ()` (Submete uma Action ao escalonador para execução assíncrona. Se anotada com `@parallel`, inicia processo de SO).  
* `channel! :: () -> (Sender::T Receiver::T)` (Cria um canal síncrono *Rendezvous* bloqueante).  
* `queue! :: Int -> (Sender::T Receiver::T)` (Cria um canal assíncrono com buffer de tamanho fixo).  
* `broadcast! :: () -> (Sender::T Subscribe::Function)` (Cria canal 1-para-N. O recetor invoca a função *Subscribe* para obter a sua fila).  
* `>! :: Sender::T T -> ()` (Operador direcional de envio. Transfere a propriedade do dado para o canal).  
* `<! :: Receiver::T -> T` (Operador direcional de receção. Bloqueia a *Green Thread* até os dados estarem disponíveis).  
* `<!? :: Receiver::T -> Optional::T` (Tentativa não-bloqueante de leitura de canal).  
* `select!` :: (Estrutura de bloco multiplexadora para aguardar em múltiplos canais simultaneamente).  
* `timeout! :: Int -> ()` (Ramo especial do `select!` para desbloqueio por inatividade em milissegundos).

2.2. Entrada e Saída (I/O)

* `echo! :: Text -> ()` (Escrita padrão na consola).  
* `input! :: Text -> Text` (Imprime a mensagem fornecida e aguarda entrada do utilizador).  
* `log! :: Text Nivel -> ()` (Registo no sistema de logs estruturado da linguagem).  
* `read! :: Text -> Array::Byte` (Lê ficheiros em memória ou como fluxo iterável).  
* `read_text! :: Text -> Text` (Lê ficheiros em memória ou como fluxo iterável).  
* `write! :: Text A -> ()` (Grava dados em recursos do sistema).

2.3. Controle de Sistema e Interrupção

* `panic! :: Text -> ()` (Aborta a execução atual imediatamente gerando log e rastreamento. Usado para estados de violação fatal ou exceções não recuperáveis).  
* `sleep! :: Int -> ()` (Pausa a execução da *Action* atual pelos milissegundos informados, libertando o escalonador).  
* `exit! :: Int -> Void` (Encerra o processo do programa integralmente retornando um código de status para o SO).  
* `now! :: () -> Int` (Retorna o carimbo de data/hora atual do sistema \- *Timestamp*).

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

* **Regra:** Todas as chamadas a `Actions` devem ser explicitamente sufixadas com um ponto de exclamação `!` (ex: `echo!`, `channel!`). Ao contrário das funções, as `Actions` podem ser variádicas, com a sintaxe (A…, B…), onde o primeiro item representa n-valores do tipo A, o segundo m-valores do tipo B .  
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

## Sistema de Tipos e Interfaces

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

## Sistema de Tipos: Primitivos e Topologia de Coleções

A linguagem Kata separa estritamente os tipos de dados básicos das estruturas de coleção. O layout de memória das coleções é ditado diretamente pelos delimitadores sintáticos (( ), \[ \], { }), permitindo que o compilador e o desenvolvedor saibam exatamente o custo de alocação em tempo de execução.

### 1\. Tipos Primitivos (Atómicos)

Os blocos de construção atómicos da linguagem são geridos de forma nativa e, na sua maioria, são tipos copiáveis de baixo custo:

* **Int / Float:** Numéricos com precisão padrão da arquitetura (ex: 64-bit).  
* **Byte:** Inteiro sem sinal de 8-bit (0-255), essencial para manipulação de I/O.  
* **Text:** Cadeias de caracteres UTF-8. Tratado como dado "cego" e puro pelo analisador léxico, sem suporte a interpolação embutida (mágica) para preservar a pureza do fluxo.  
* **Unit:** Representado também pelo literal tuplo vazio (), indica a ausência matemática de valor, substituindo conceitos como void.

### 2\. Coleções e Layout de Memória

A linguagem divide as coleções em quatro categorias fundamentadas em três eixos: tipagem (homogénea vs. heterogénea), mutabilidade de tamanho (dinâmico vs. fixo) e topologia de alocação de memória.

#### 2.1. Tuplos (Heterogêneo, Tamanho Fixo)

* **Sintaxe:** Parênteses ( ).  
* **Comportamento:** Estruturas heterogéneas utilizadas para agrupamento posicional estrito.  
* **Mecânica do Compilador:** Como definido pela "Teoria Unificada", qualquer agrupamento isolado por parênteses que não seja resolvido como uma chamada de função torna-se um tuplo literal estrutural.

#### 2.2. Listas Persistentes (Homogêneo, Tamanho Dinâmico)

* **Sintaxe:** Parênteses retos \[ \].  
* **Assinatura:** List::T.  
* **Comportamento:** Otimizadas para algoritmos funcionais puramente recursivos. Operam sob uma topologia encadeada, garantindo imutabilidade de custo zero através da partilha estrutural de caudas (Cons de cabeças e caudas).

#### 2.3. Arrays Contíguos (Homogêneo, Tamanho Dinâmico)

* **Sintaxe:** Chaves sem terminadores ; (ex: {1 2 3}).  
* **Assinatura:** Array::T.  
* **Comportamento:** Estruturas flexíveis de I/O em bloco. Ocupam blocos de memória contígua garantindo o máximo aproveitamento da cache da CPU (Cache-Friendly), permitindo iterações rápidas.  
* **Restrição Matemática:** Por terem dimensões desconhecidas no momento da compilação, o *Type Checker* **não** desbloqueia operadores matemáticos avançados de álgebra linear para Arrays.

#### 2.4. Tensores Estáticos (Homogêneo numérico, Tamanho Fixo N-Dimensional)

* **Sintaxe:** Chaves com a presença do promotor ; para forçar quebra de dimensões (ex: vetor linha {1 2 3 ;} ou matriz {1 2 ; 3 4}).  
* **Assinatura:** Tensor::T::(Int…).  
* **Comportamento:** A família de elite para processamento matemático acelerado. O seu tamanho e dimensionalidade são conhecidos em tempo de compilação (*Const Generics*). O *Type Checker* desbloqueia regras matemáticas rigorosas (ADD\_BEHAVIOR, MUL\_BEHAVIOR, `DOT_BEHAVIOR`), traduzindo estas operações diretamente para instruções SIMD no motor Cranelift sem qualquer *overhead*.

### 3\. Tipos Algébricos de Dados (ADTs)

A Kata-Lang não possui o conceito de classes ou herança orientada a objetos. A modelagem de domínio é feita estritamente através de Tipos Algébricos de Dados, divididos em Tipos Produto (Estruturas/Registos) e Tipos Soma (Variantes/Enums).

A linguagem utiliza duas palavras-chave distintas para a declaração, garantindo desambiguação léxica imediata: `data` para conjunções e `enum` para disjunções.

#### 3.1. Tipos Produto (Estruturas Lógicas)

Representam a conjunção lógica (AND). Uma instância de um Tipo Produto contém simultaneamente todos os campos declarados. A topologia de memória é a de um bloco contíguo alocado (Struct).

* **Sintaxe:** Declarados com a palavra-chave `data`. Os campos ficam entre parênteses, separados por espaços. A anotação de tipo é feita com `::` (opcional caso o compilador consiga inferir, mas recomendada para documentação de domínio).  
* **Acesso:** Os campos são acedidos via notação de ponto (`.`).  
  `# Produto: Um Vetor2D possui um 'x' E um 'y'`  
  `data Vetor2D (x::Float y::Float)`  
    
  `# Produto genérico`  
  `data Caixa::T (conteudo::T peso::Int)`  
    
  `action processar_vetor`  
      `let v (Vetor2D 10.5 20.0)`  
      `echo! "Eixo X: #{v.x}"`

*(Nota: Tuplos `(A B)` são, na sua essência, Tipos Produto anónimos sem chaves nomeadas).*

#### 3.2. Tipos Soma (Variantes)

Representam a disjunção lógica (OR). Uma instância de um Tipo Soma ocupa o tamanho em memória da sua maior variante, mais uma *tag* de identificação discriminatória (Discriminant Tag). O compilador garante a segurança de acesso obrigando o uso de *Pattern Matching* em `Functions` ou `match` em `Actions`.

* **Sintaxe:** Declarados com a palavra-chave `enum`. As variantes são separadas pelo operador `|`. Podem ser unitárias (sem carga de dados) ou carregar tipos associados. O uso de múltiplas linhas é encorajado para clareza visual.

\# Soma: Uma Transação é Aprovada, OU Recusada (com um motivo), OU Pendente

`enum Transacao`

    `Aprovada` 

    `| Recusada::Text` 

    `| Pendente`

`action verificar_pagamento (t::Transacao)`

    `match t`

        `Aprovada: echo! "Sucesso"`

        `Recusada motivo: echo! "Falha: #{motivo}"`

        `Pendente: echo! "Aguardando processamento"`

### 3.3. Tipos Soma Fundamentais (Standard Library)

Para eliminar comportamentos especiais (mágica de compilador), três conceitos fundamentais de controlo de fluxo são definidos nativamente como Tipos Soma na biblioteca padrão utilizando a palavra-chave `enum`.

#### **3.3.1. `Bool` (Lógica Booleana)** 

O tipo primitivo `Bool` é uma variante simples. Isto significa que guardas e lógicas de desvio não requerem código de máquina especial no compilador; utilizam a mesma infraestrutura de análise de exaustividade de qualquer outro Tipo Soma.

`enum Bool (True | False)`

#### **3.3.2. `Optional::T` (Ausência Segura de Valor)** 

A linguagem não possui o conceito de `null` ou `nil` (o "erro de mil milhões de dólares"). A ausência de valor é semanticamente explícita no sistema de tipos através do `Optional`. Funções que podem não encontrar um resultado (como a busca numa lista) devem retornar este tipo.

`enum Optional::T (Some::T | None)`

#### **3.3.3. `Result::T::E` (Tratamento de Falhas)** 

Não existe mecanismo de lançamento de exceções (`try/catch`) na linguagem (exceções invisíveis quebram a pureza funcional). Operações passíveis de falha (I/O, conversões dinâmicas de tipos, divisão por zero em tempo de execução) retornam o tipo `Result`. O programador é forçado a lidar estaticamente com o cenário de sucesso (`Ok`) e o de falha (`Err`).

`enum Result::T::E (Ok::T | Err::E)`

### Fronteira Dinâmica para Estática (Cast Seguro):

A conversão de dados flexíveis (Array::T) adquiridos do I/O para dados matemáticos rígidos (Tensor::T::(Int…)) não é implícita. Deve ser feita usando a chamada ao tipo como um construtor de coerção (ex: `Tensor::Int::(3 3) dados_dinamicos`) em *runtime*, o qual sempre retorna um Result, forçando o programador a lidar com uma eventual falha na incompatibilidade de dimensões de forma pura antes da execução do cálculo matemático.

## Controle de Fluxo: Funcional e Imperativo

A linguagem Kata elimina estruturas de controle ambíguas, dividindo as ferramentas de fluxo estritamente pelos domínios em que operam: Funcional (puro, declarativo) e Imperativo (impuro, baseado em estado).

### 1\. Controle de Fluxo Funcional (Domínio Puro)

No domínio das **Functions**, não existem laços de repetição imperativos (`for`, `while`) nem a palavra-chave `if`. O fluxo é direcionado unicamente por casamento de padrões, guardas lógicas e recursão otimizada.

#### **1.1. Pattern Matching (Despacho de Lambda)** 

O controle de fluxo primário é feito pela assinatura do `lambda`. Uma função pode ser composta por múltiplas definições de lambda. O compilador avaliará os argumentos de cima para baixo e executará o primeiro corpo cujo padrão estrutural (*Pattern*) corresponda à entrada.

fibonacci :: Int \-\> Int  
lambda (0) 0             \# Match exato literal  
lambda (1) 1  
lambda (n) \+ (fibonacci (- n 1)) (fibonacci (- n 2))

#### **1.2. Guards (Condicionais Puras)** 

Para desvios lógicos que não podem ser resolvidos por *Pattern Matching* estrutural, utilizam-se os *Guards*. Eles substituem as cadeias de `if/else`, operando como testes booleanos sucessivos (avaliados de cima para baixo). O sufixo `:` separa a condição do resultado.

max :: Int Int \-\> Int  
lambda (x y)  
    \> x y: x  
    otherwise: y

#### 1.3. O Escopo Local: 

**`let` vs `with`** A linguagem provê dois mecanismos estritos para amarração de nomes (*bindings*) no escopo funcional, operando em direções opostas de avaliação:

* **`let` (Avaliação Top-Down):** Define uma variável imutável no fluxo léxico antes de seu uso. Utilizado para cálculos sequenciais no corpo do lambda.  
* **`with` (Avaliação Bottom-Up / Metadados):** Uma cláusula declarativa fixada no final do lambda. Possui duas finalidades exclusivas:  
  1. Resolver computações requeridas pelos *Guards* antes que as condições sejam testadas.  
  2. Anexar restrições de Tipos Genéricos (Contratos de Interface).

processar :: T \-\> Int  
lambda (entrada)  
    \# 'let' para computação sequencial  
    let base (calcular\_base entrada)   
      
    \# Guards consumindo as variáveis declaradas no 'with'  
    \> variante 10: \+ base variante  
    otherwise: base  
      
    \# 'with' definindo as dependências lógicas e o Tipo Genérico  
    with   
        variante as (extrair\_variante entrada)  
        T as (T implements ORD)

### 2\. Controle de Fluxo Imperativo (Actions)

O domínio das **Actions** é projetado para interação com o Sistema Operacional e concorrência. Aqui, a recursividade é banida e a desestruturação segura é imposta.

#### 2.1. A Proibição de Recursão (Hard Error) 

As *Actions* operam sobre um escalonador de *Green Threads* em tempo de execução (compilado para máquinas de estado sob o *Cranelift/Tokio*). **É expressamente proibido realizar chamadas recursivas dentro de uma Action.** Como o escalonador não pode garantir *Tail Call Optimization* (TCO) para esse contexto impuro, qualquer detecção de ciclo recursivo pelo *Type Checker* resultará em um **Erro Fatal de Compilação**, prevenindo *Stack Overflows*.

#### 2.2. Laços de Repetição 

**(`loop` e `for`)** Para substituir a recursão em operações de I/O, as *Actions* utilizam primitivas iterativas clássicas com as chaves de controle `break` (interrompe o laço) e `continue` (avança a iteração).

* **`loop`**: Um laço infinito fundamental.  
* **`for elemento colecao`**: Um iterador seguro que consome estruturas que implementam a interface nativa `ITERABLE` (Listas, Arrays, Ranges). A sintaxe é estritamente posicional, alinhando-se com a declaração de variáveis (`let`), onde o espaço delimita a variável de captura da fonte de dados.

Abaixo, um exemplo da utilização do `loop` coordenando estado mutável (`var`), saltos de iteração (`continue`) e condição de saída (`break`):

`action conectar_servidor`  
    `var tentativas 0`  
      
    `loop`  
        `var tentativas (+ tentativas 1)`  
          
        `# Limite de segurança para evitar loops infinitos acidentais`  
        `match (> tentativas 5)`  
            `True:`  
                `echo! "Limite de tentativas excedido. Abortando."`  
                `break`  
            `False:`  
                `() # Continua a execução normalmente`  
          
        `let pronto (ping!)`  
          
        `match pronto`  
            `False:`  
                `echo! "Servidor indisponível. Aguardando..."`  
                `sleep! 1000`  
                `continue  # Pula o restante do bloco e inicia nova iteração`  
            `True:`  
                `echo! "Conexão estabelecida com sucesso!"`  
                `break     # Sai do loop imediatamente`

Abaixo, um exemplo da utilização do laço `for` demonstrando a sintaxe puramente posicional e a iteração sobre uma coleção de dados:

`action processar_lote (lista_usuarios)`  
    `for usuario lista_usuarios`  
        `match (= usuario.status "inativo")`  
            `True:`   
                `continue  # Pula iteração para utilizadores inativos`  
            `False:`   
                `echo! "Processando utilizador: #{usuario.nome}"`

#### **2.3. Desestruturação Exaustiva (`match`)** 

A palavra-chave `if` não existe na linguagem Kata. O desvio condicional dentro do mundo imperativo é centralizado no bloco `match`. O `match` é obrigatório para lidar com Tipos de Soma (como `Result::T::E` e `Optional::T`), e deve ser **estritamente exaustivo**. O compilador recusará qualquer programa que não cubra todas as variantes possíveis da estrutura, a menos que a cláusula de fallback `otherwise:` seja fornecida.

action ler\_banco (id\_usuario)  
    let resposta (db\_query\! id\_usuario)  
      
    match resposta  
        \# Extração de valores em caso de sucesso  
        Ok dados: echo\! "Nome: \#{dados.nome}"  
          
        \# O compilador força o tratamento do 'Err', prevenindo crashes silenciosos  
        Err falha:   
            log\! "Falha no banco: \#{falha}"  
            panic\! "Abortando ação."

O `match` também pode ser utilizado para testar expressões booleanas de forma exaustiva:

`action validar_estado`  
    `let ativo (verificar_status!)`  
      
    `match ativo`  
        `True: echo! "Prosseguindo..."`  
        `False: break`

## Sistema de Módulos, Visibilidade e Coerência

O sistema de pacotes da Kata-Lang é concebido para evitar ambiguidade de resolução de símbolos e prevenir o "Inferno de Dependências" (onde o comportamento de uma aplicação muda drasticamente dependendo da ordem de importação de bibliotecas).

### 1\. Encapsulamento, Visibilidade e Importação

* **A Unidade Base:** Cada ficheiro `.kata` representa um módulo isolado.  
* **Privacidade por Predefinição:** Qualquer tipo (`data`/`enum`), função (`lambda`) ou ação (`action`) declarada num ficheiro é estritamente invisível para o exterior.  
* **A Cláusula `export`:** Apenas identificadores explicitamente listados após a palavra-chave `export` são acessíveis por outros módulos. A cláusula `export` é uma lista de nomes, não um bloco de delimitação léxica.  
  `# modulo_matematico.kata`  
  `let pi 3.14159 # Privado`  
    
  `soma :: Int Int -> Int`  
  `lambda (x y) + x y`  
    
  `export soma`

* **Importação e Namespaces:** A linguagem suporta duas mecânicas estritas de importação para evitar colisões de identificadores:  
  * **Importação por Namespace:** Ao importar um módulo inteiro (`import biblioteca`), todos os identificadores exportados por ele ficam confinados ao *namespace* do próprio módulo, sendo acedidos via notação de ponto (ex: `biblioteca.funcao`).  
  * **Importação Unitária:** Para importar um único identificador diretamente para o escopo léxico atual, utiliza-se a notação de ponto na própria diretiva (`import biblioteca.Item`).

  `import sistema_arquivos        # Importa o namespace completo`

  `import modulo_matematico.soma  # Importação unitária`


  `action principal`

      `let fd (sistema_arquivos.abrir! "config.txt") # Uso com namespace`

      `let calc (soma 10 20)                         # Uso direto no escopo`

### 2\. Declaração de Contratos (`implements`)

A implementação de Interfaces (Polimorfismo Ad-Hoc e Despacho Múltiplo) em Kata **não é encapsulável**. As declarações `implements` são contratos globais ao nível do módulo. Elas devem ser declaradas no nível superior (*top-level*) do ficheiro, nunca aninhadas dentro de controlos de fluxo ou blocos de visibilidade.

Quando um módulo importa um Tipo de Dado, **importa irrevogavelmente todas as implementações de interface** atreladas a esse tipo que estejam visíveis no módulo de origem.

`data Vec2 (x y)`

`# A implementação é feita no topo do escopo. Não requer a palavra 'export'.`  
`# Se 'Vec2' for exportado, este contrato viaja com ele automaticamente.`  
`Vec2 implements NUM`  
    `+ :: Vec2 Vec2 -> Vec2`  
    `lambda (a b) Vec2 (+ a.x b.x) (+ a.y b.y)`  
      
    `- :: Vec2 Vec2 -> Vec2`  
    `lambda (a b) Vec2 (- a.x b.x) (- a.y b.y)`

### 3\. A Regra de Coerência (Orphan Rule)

Para evitar que duas bibliotecas de terceiros implementem a mesma interface para o mesmo tipo de forma contraditória, o *Type Checker* impõe a **Regra de Coerência** no ato da compilação:

*Para implementar uma Interface para um Tipo, **pelo menos um dos dois** (a Interface ou o Tipo) tem de ter sido definido no módulo atual.*

* **Permitido:** Tipo Local \+ Interface Local.  
* **Permitido:** Tipo Local \+ Interface Externa.  
* **Permitido:** Tipo Externo \+ Interface Local.  
* **Proibido (Falha de Compilação):** Tipo Externo \+ Interface Externa (Instância Órfã).

**Resolução via Padrão *Newtype* (`alias`):** Caso seja estritamente necessário implementar uma Interface Externa num Tipo Externo, o programador deve encapsular o tipo externo numa nova estrutura definida localmente utilizando a palavra-chave `alias`. O compilador tratará isto como um novo tipo nominal estrito (satisfazendo a Regra de Coerência), mas eliminará o custo de alocação deste invólucro em tempo de execução (*Zero-Cost Abstraction*).

No exemplo abaixo, a biblioteca JSON é importada como um *namespace*, forçando o uso do prefixo na interface, enquanto a Matriz é importada unitariamente para uso direto.

`import biblioteca_json           # Importação por Namespace`  
`import biblioteca_math.Matrix    # Importação Unitária`

`# 1. Cria-se o Newtype Local (Alias Nominal Forte)`  
`alias MatrizLocal Matrix`

`# 2. Implementa-se a interface externa (sob o namespace) no tipo local`  
`MatrizLocal implements biblioteca_json.JsonSerializable`  
    `to_json :: MatrizLocal -> Text`  
    `lambda (mat) …`

## Concorrência e Isolamento (Modelo CSP)

A Kata-Lang implementa concorrência exclusivamente no domínio das **Actions**. O modelo baseia-se em *Communicating Sequential Processes* (CSP). Não existe partilha de memória entre processos paralelos; o isolamento é absoluto e a sincronização ocorre estritamente através da passagem de mensagens por canais tipados.

### `1. Criação de Processos (fork!)`

A primitiva fork\! aceita a invocação de uma Action e submete-a ao escalonador do *runtime*. Por predefinição, a execução ocorre numa *Green Thread* cooperativa (M:N).

`action worker (id)`  
    `echo! "Worker #{id} a iniciar"`

`action main`  
    `# Inicia a execução concorrente e liberta a thread atual imediatamente`  
    `fork! (worker 1)`

### 2\. Topologia de Canais

Os canais são o único meio de comunicação inter-processos. São unidirecionais na sua utilização, mas a sua criação devolve sempre um tuplo contendo o lado emissor (tx) e o lado recetor (rx). A linguagem oferece três topologias com garantias de bloqueio distintas:

#### 2.1. Canal Rendezvous (channel\!)

 Síncrono e sem *buffer*. A transferência de dados exige que o emissor e o recetor estejam prontos simultaneamente.

* O operador de envio \>\! bloqueia até que uma *Action* execute uma receção \<\!.  
* O operador de receção \<\! bloqueia até que uma *Action* execute um envio \>\!.

`let (tx rx) channel!()`

#### **2.2. Fila Assíncrona (queue\!)** 

Possui um *buffer* de tamanho fixo em memória. Funciona como mecanismo primário de *backpressure* (contrapressão) para ritmos desiguais de I/O.

* O envio \>\! bloqueia a *Action* emissora caso o *buffer* atinja o seu limite, forçando-a a aguardar (travada) até que um recetor consuma pelo menos um item e liberte espaço na fila.  
* A receção \<\! bloqueia a *Action* recetora apenas se o *buffer* estiver completamente vazio.

`let (tx rx) queue!(16) # Fila com capacidade máxima para 16 elementos`

#### **2.3. Difusão Múltipla (broadcast\!)** 

Topologia *Publish-Subscribe* (1 para N).

* A criação devolve um emissor (tx) e uma **fábrica de recetores** (subscribe).  
* **Sem retroatividade (*No Replay*):** Quando um novo recetor é inscrito através de subscribe, ele não tem acesso ao histórico de mensagens passado. Receberá estritamente as mensagens publicadas após o momento exato da sua inscrição.  
* O envio \>\! **nunca** bloqueia o emissor. Se um recetor específico estiver demasiado atrasado a ponto de encher o seu *buffer* local associado à subscrição, as mensagens mais antigas desse recetor são descartadas silenciosamente (*Drop-Oldest*), garantindo que o publicador nunca é penalizado por subscritores lentos.  
  `let (tx subscribe) broadcast!()`  
  `let rx_cliente_1 subscribe(4) # Subscreve com um buffer local de 4`  
  `let rx_cliente_2 subscribe(8) # Receberá apenas os envios feitos a partir desta linha`

### 3\. Operações de Comunicação

A interação com os canais utiliza a notação prefixada e os operadores direcionais de I/O.

* \>\! tx valor: Transfere a propriedade do valor para o canal.  
* \<\! rx: Extrai o próximo valor disponível do canal.  
  `action produtor (tx)`  
      `>! tx "Dados críticos"`  
    
  `action consumidor (rx)`  
      `let dados (<! rx)`  
      `echo! dados`

### 4\. Multiplexagem Não-Determinística (select\!)

O select\! é a estrutura de controlo imperativa para aguardar múltiplos eventos assíncronos. Avalia todas as operações de canal declaradas nos seus ramos (case) e bloqueia a *Action* até que **um** dos eventos esteja pronto. Se múltiplos canais estiverem prontos em simultâneo, o escalonador escolhe um ramo de forma pseudoaleatória para garantir justiça (*fairness*) e evitar a inanição (*starvation*) de canais secundários.

`action multiplexador (rx_a rx_b tx_c)`  
    `loop`  
        `select!`  
            `# Ramo de receção`  
            `case (<! rx_a) -> valor_a:`  
                `echo! "Recebido de A: #{valor_a}"`  
              
            `# Ramo de receção alternativo`  
            `case (<! rx_b) -> valor_b:`  
                `echo! "Recebido de B: #{valor_b}"`  
              
            `# Ramo de emissão (só executa se tx_c tiver espaço no buffer)`  
            `case (>! tx_c "Ping"):`  
                `echo! "Sinal enviado para C"`  
              
            `# Desbloqueio temporal`  
            `timeout! 1000:`  
                `echo! "Inatividade detetada. 1s passado."`

## Diretivas de Compilação e Runtime

As diretivas são anotações sintáticas prefixadas pelo símbolo `@`. Atuam como instruções explícitas para o compilador e para o *runtime*, alterando características não-funcionais do código (como alocação de memória, agendamento de concorrência ou ligação a bibliotecas externas) sem modificar a semântica matemática do programa.

### 1\. Isolamento de Processo (`@parallel`)

Por predefinição, a invocação de `fork!` submete uma *Action* ao escalonador de *Green Threads* em modo cooperativo dentro do mesmo processo. A diretiva `@parallel` força o *runtime* a instanciar um **Processo Nativo do Sistema Operativo** (Thread Pesada) isolado.

* **Indicação:** Deve ser usada estritamente para tarefas com uso intensivo de CPU (CPU-bound) que bloqueariam o escalonador cooperativo, ou para obter isolamento de memória absoluto.  
* **Restrição de Transporte (IPC):** A comunicação com uma *Action* anotada com `@parallel` é feita via canais, o que obriga à serialização dos dados (IPC). É **estritamente proibido** enviar "Recursos Vinculados" (System Handles, como descritores de ficheiros ou sockets) através de canais ligados a processos `@parallel`, uma vez que estes não são serializáveis e dependem do contexto de memória do processo original.  
  `@parallel`  
  `action processar_video (rx_frames tx_resultado)`  
      `# Executa num processo nativo isolado pelo S.O.`  
      `let frame (recv! rx_frames)`  
      `# ... processamento intensivo ...`

### 2\. Memoização Automática (`@cache_strategy`)

Aplica-se **exclusivamente a Funções (puras)**. Como as funções em Kata são deterministicamente puras, o compilador pode interceptar a invocação e devolver um resultado previamente calculado armazenado numa tabela de dispersão (*Hash Table* global partilhada), evitando recomputações dispendiosas.

Por predefinição, quando anotada sem argumentos, a estratégia de cache ativada é um `lru` (*Least Recently Used*) com uma heurística de retenção gerida pelo próprio compilador. O programador pode, contudo, definir configurações exatas explicitamente.

A diretiva recebe os seus argumentos através de um bloco de chaves `{ }`:

* `strategy`: Estratégia de substituição explícita (`'lru'`, `'lfu'`, ou `'none'` para forçar a desativação da memoização nalgum contexto específico).  
* `size`: Limite absoluto de entradas no cache.  
* `ttl`: Tempo de vida das entradas em milissegundos (opcional).  
  `@cache_strategy{strategy: 'lru', size: 1000}`  
  `fibonacci :: Int -> Int`  
  `lambda (0) 0`  
  `lambda (1) 1`  
  `lambda (n) + (fibonacci (- n 1)) (fibonacci (- n 2))`

### 3\. Foreign Function Interface (`@ffi`)

Utilizada para ligar assinaturas de funções Kata a símbolos compilados externamente (em C ou Rust). É a válvula de escape do sistema de tipos, permitindo invocar primitivas de sistema. O compilador confia na pureza declarada na assinatura. Se a função chamada através do FFI introduzir efeitos colaterais e for anotada como uma Função pura (em vez de uma *Action*), a responsabilidade por falhas de corrupção de memória é inteiramente do programador.

`# O compilador não gera o corpo desta Action, apenas cria a ligação no Linker`  
`@ffi('kata_rt_print')`  
`echo! :: Text -> Action::Unit`

### 4\. Execução em Tempo de Compilação (`@comptime`)

A diretiva `@comptime` instrui o compilador a avaliar a função ou bloco de código estritamente durante a fase de compilação (*Compile-Time Function Execution*).

* **Substituição de Macros:** Por basear-se em funções estritamente puras, o `@comptime` elimina a necessidade de um sistema de macros separado. O código é executado pela própria infraestrutura do compilador (ou interpretador interno), e o seu resultado final é embutido como um literal estático (constante) no binário gerado.  
* **Restrições:** O código `@comptime` só pode operar sobre dados que sejam inteiramente conhecidos em tempo de compilação (como literais estruturais, ranges estáticos, ou os resultados de outras funções `@comptime`). É impossível utilizar blocos de *Action* (I/O) ou canais concorrentes durante a avaliação desta diretiva.  
  `@comptime`  
  `gerar_tabela_senos :: Int -> Array::Float`  
  `lambda (precisao) ...`  
  `# O resultado desta computação será chumbado no binário`  
  `# como um literal Array::Float sem qualquer custo de runtime.`

## 