# **Especificação Técnica: Kata-Lang (Revisão v2.1)**

## **1\. Filosofia e Arquitetura**

A linguagem opera sobre uma segregação rígida de responsabilidades para garantir segurança e previsibilidade, inspirada no modelo de processamento distribuído (Spark) e programação funcional.

### **O Tríade de Separação:**

1. **Dados (Data):** Estruturas de informação imutáveis e serializáveis.  
2. **Cálculos (Functions):** Transformações puras e determinísticas. Funções constroem um grafo de execução (DAG), mas não executam o processamento pesado imediatamente. Elas operam estritamente sobre dados e não capturam estado externo.  
3. **Ações (Actions):** O motor de execução. Código impuro, ansioso (eager) e capaz de mutação local. Actions são o único lugar onde recursos do sistema (arquivos, sockets) podem existir.

### **Modelo de Execução:**

* **Lazy Streams:** As coleções de dados operam como fluxos. A definição de uma lista ou leitura de arquivo não carrega os dados na memória, apenas prepara o ponteiro.  
* **Strict Operations:** As operações aritméticas e lógicas dentro das funções são estritas. Ao processar um item do fluxo, o cálculo é resolvido imediatamente, evitando a criação de *thunks* (promessas) na memória e prevenindo estouro de pilha/heap.

## **2\. Sintaxe e Notação**

A linguagem utiliza **notação prefixada (Polish Notation)** obrigatória para eliminar a complexidade de precedência de operadores e ambiguidades de parsing.  
**Regra de Ouro do Parser:** O espaço em branco é o delimitador funcional estrito.

* \+ 1 1 é uma soma.  
* \+1 1 é interpretado como identificador de função (+1) aplicado a 1\.  
* \- 10 é uma subtração (esperando argumento).  
* \-10 é o literal numérico negativo.

### **Convenções de Identificadores**

O compilador utiliza a capitalização para desambiguação léxica imediata:

* **Interfaces (ALL\_CAPS):** Denotam contratos e restrições universais (ex: NUM, ORD, EQ).  
* **Tipos (CamelCase):** Denotam estruturas de dados concretas ou parâmetros genéricos (ex: Int, Vector, T).  
* **Funções e Variáveis (snake\_case):** Denotam valores e computações (ex: soma\_vetor, meu\_valor, map).

### **Justificativa de Design: Espaços e Notação Prefixa**

*Nota do Autor:* A notação prefixa é chocante à primeira vista, afinal toda educação em matemática usa a notação infixa. Mas misturamos infixa e prefixa (f(g(x)) \+ x), o que *é* confuso, apenas parece “lugar comum”.   
O motivo principal é a **implementação de precedência**. Notação infixa exige regras complexas e organizadores (parênteses) essenciais para a máquina. Na prefixa, eles são irrelevantes para a execução. Além disso, exige uso correto de espaços: \+ 1 1 é válido, \+1 1 é outra função. Isso libera os símbolos \+ e \- para números (-10) sem ambiguidade com operadores (- 10).

### **Funções Nomeadas e Estrutura**

Uma função nomeada é composta por:

1. Decorators (Opcionais)  
2. Nome da Função  
3. Assinatura de Tipo (Recomendada, mas opcional devido à inferência)  
4. Um ou mais corpos definidos por Lambdas (λ)

`# Exemplo: FizzBuzz`  
`fizzbuzz :: Int Int Int -> Text`  
`λ (fizz_num buzz_num x)`  
    `both: "FizzBuzz"`  
    `fizz: "Fizz"`  
    `buzz: "Buzz"`  
    `otherwise: str x`  
    `with fizz as = (mod x fizz_num) 0`  
         `buzz as = (mod x buzz_num) 0`   
         `both as (and fizz buzz)`

### **Pattern Matching (Polimorfismo de Função)**

O despacho da função é decidido pelo padrão dos argumentos. Uma mesma função pode ter múltiplas definições de λ, e a primeira que satisfizer o padrão será executada.  
`# Exemplo: Fibonacci com Pattern Matching`  
`fibonacci`  
`λ (2 x y)`   
    `+ x y             # Se 1º arg for 2, soma os outros dois`  
`λ (n x y)`   
    `fibonacci (- n 1, y, + x y) # Recursão de cauda`

### **Guards (Guardas)**

Substituem condicionais if/else. Executam computação para decidir o fluxo dentro de um Lambda.  
`# Exemplo genérico com Guards`  
`minha_funcao`  
`λ (x y)`  
    `> a 0: + y a        # Se 'a' > 0, execute isto`  
    `= b 0: + x b        # Senão, se 'b' == 0, execute isto`  
    `otherwise: y        # Caso contrário`  
    `with a as - x 1`  
         `b as * y x`

### **Captura de Variáveis (Closures e Move Semantics)**

Funções podem capturar variáveis do escopo léxico onde foram definidas. Devido à imutabilidade dos dados em Kata-Lang, essa captura segue a semântica de **Snapshot (Cópia/Move)**.

* **Mecânica:** Quando um lambda usa uma variável externa, o valor dessa variável é copiado para dentro da estrutura da closure no momento da criação.  
* **Serialização:** Como **Functions** não podem conter System Handles (recursos não-serializáveis), qualquer closure criada dentro de uma função é garantidamente serializável e segura para envio entre processos (@parallel).

`λ (x)`  
    `let y 10`  
    `# Permitido: O valor de 'y' (10) é capturado pela closure.`  
    `# A closure torna-se um pacote autocontido {code_ptr, env: {y: 10}}`  
    `map (λ (z) + z y) lista` 

### **Aplicação Parcial (Currying Explícito), Piping e Composição**

Devido à natureza estrita e à sobrecarga de operadores, o Currying não é automático (implícito). O compilador exige que a intenção de criar uma função parcial seja explícita para evitar erros de aridade.

#### **1\. Aplicação Parcial (?)**

O símbolo ? atua como um placeholder para argumentos que serão fornecidos posteriormente. Isso cria uma nova função (closure manual).  
`# Função base`  
`soma :: Int Int -> Int`  
`λ (x y) + x y`

`# Criando uma função parcial (soma_10)`  
`# O '?' indica o argumento faltante.`  
`let soma_10 (soma 10 ?)`

`# Uso`  
`soma_10 5  # Resultado: 15`

#### **2\. Piping (|\>)**

O operador de pipe melhora a legibilidade de chamadas aninhadas, passando o resultado da expressão à esquerda para o local marcado com ? na expressão à direita.  
`# Sem Pipe (Leitura de dentro para fora, difícil em notação prefixa)`  
`# Equivalente a: f3(f2(f1(x)))`  
`f3 (f2 (f1 x))`

`# Com Pipe (Leitura linear da esquerda para a direita)`  
`x |> f1 ? |> f2 ? |> f3 ?`

#### **3\. Composição de Funções (°)**

Cria uma nova função combinando duas existentes matematicamente, sem executá-las imediatamente.  
`# Definição: h(x) = f(g(x))`  
`let h (f ° g)`

## **3\. Sistema de Tipos e Verificação**

Tipagem forte, inferência estática onde possível, com suporte a **Tipo-Dependência** (Refinement Types) e **Generics**.

### **Tipo-Dependência e Smart Constructors**

A linguagem permite restringir valores diretamente na assinatura do tipo. **Sintaxe:** |TipoBase, Condição1, Condição2...|.  
`# Definição de um inteiro que deve ser positivo`  
`data PositiveInt as |Int, > ? 0|`

**Mecânica de Validação:**

* **Compile-Time:** Literais constantes são verificados pelo compilador.  
* **Runtime (Actions):** Dados externos devem passar por tratamento obrigatório via Pattern Matching de Result ou Optional.

### **Restrição de Recursos (System Handles)**

Tipos que representam recursos do sistema (Arquivos, Sockets, Conexões de BD) são classificados como **Recursos Vinculados**.

* **Restrição de Escopo:** Eles só podem existir dentro de **Actions**. É um erro de compilação passar um recurso como argumento para uma **Function** pura.  
* **Motivação:** Funções devem operar apenas sobre dados imutáveis e serializáveis. A manipulação de recursos é inerentemente impura e dependente do estado do sistema.

### **Generics e Polimorfismo Paramétrico**

Para evitar duplicação de lógica, a linguagem suporta Polimorfismo Paramétrico com **Monomorfização** (o compilador gera código especializado para cada tipo concreto utilizado, garantindo performance zero-cost).  
**Sintaxe:** Variáveis de tipo (geralmente letras maiúsculas simples como T, U) são usadas na assinatura. As restrições (Constraints) são definidas na cláusula with ao final da função.  
`# Função Genérica de Máximo`  
`# Aceita dois valores do mesmo tipo T e retorna um T`  
`max :: T T -> T`  
`λ (x y)`  
    `> x y: x`  
    `otherwise: y`  
`with T as |T implements ORD|`

**Interação com Tipo-Dependência:** É possível combinar Generics com Refinement Types para criar restrições poderosas e reutilizáveis.  
`# Soma de lista contendo apenas números positivos`  
`# A restrição exige que T seja numérico E maior que zero`  
`sum_positives :: List::T -> T`  
`λ (lista) reduce (+, lista)`  
`with T as |T implements NUM, > ? 0|`

**Swap de Tuplas (Múltiplos Genéricos):**  
`swap :: (A, B) -> (B, A)`  
`λ (tuple)`  
    `let (a, b) as tuple`  
    `(b, a)`  
`with A as |A|, B as |B|  # |T| vazio significa "qualquer tipo"`

## **4\. Interfaces e Sobrecarga de Operadores**

Em Kata-Lang, operadores (+, \-, \*, \=, etc.) são **funções normais** definidas em interfaces padrão (NUM, ORD, EQ).

### **Mecânica de Sobrecarga (Despacho Múltiplo)**

Não existem métodos atrelados a classes (ex: obj.add(b)). A sobrecarga ocorre definindo novas implementações para uma função existente da interface, especificando os tipos concretos nos argumentos.  
**Regra de Implementação:** Se um tipo declara implements INTERFACE, ele **deve** fornecer implementações para todas as funções listadas na interface, cobrindo os casos onde o tipo interage consigo mesmo.

### **Exemplo: Vetor 2D e Interface NUM**

A interface NUM exige: \+, \-, \*, /.  
`# 1. Definição do Tipo`  
`data Vec2 (x, y) implements NUM`

`# 2. Implementação dos Operadores (Sobrecarga)`

`# Soma (Vec2 + Vec2)`  
`λ (+ a::Vec2 b::Vec2)`  
    `Vec2 (+ a.x b.x) (+ a.y b.y)`

`# Subtração (Vec2 - Vec2)`  
`λ (- a::Vec2 b::Vec2)`  
    `Vec2 (- a.x b.x) (- a.y b.y)`

`# Multiplicação Escalar (Vec2 * NUM) - Operação Heterogênea`  
`λ (* a::Vec2 k::NUM)`  
    `Vec2 (* a.x k) (* a.y k)`

`# Comutatividade deve ser explícita se desejada (NUM * Vec2)`  
`λ (* k::NUM a::Vec2)`  
    `* a k  # Reutiliza a definição anterior`

### **Resolução de Ambiguidade**

Como a notação é prefixada (+ a b), o compilador verifica os tipos de a e b.

1. Busca uma implementação exata: λ (+ ::Vec2 ::Vec2).  
2. Se não encontrar, busca implementações genéricas ou interfaces: λ (+ ::NUM ::NUM).  
3. Se houver ambiguidade ou nenhuma implementação, erro de compilação: *"No implementation of function '+' matches signature (Vec2, Matrix)"*.

## **5\. Actions, Erros e Efeitos Colaterais**

Actions são o único local onde I/O, concorrência e mutabilidade existem.

### **Controle de Fluxo Imperativo**

Como **Actions** não devem utilizar recursão (para evitar estouro de pilha no runtime imperativo), a linguagem oferece primitivas robustas de repetição.

#### **1\. Laço Infinito (loop)**

A primitiva mais básica, equivalente a while(true).

* **break**: Sai do laço imediatamente.  
* **continue**: Pula para a próxima iteração.

`action monitor`  
    `loop`  
        `let status check_system!`  
        `if (critical? status) break`  
        `sleep! 1000`

#### **2\. Laço de Iteração (for)**

Itera sobre qualquer estrutura que implemente a interface Iterable (Listas, Vetores, Ranges, Streams). Suporta destruturação automática.  
**Sintaxe:** for elemento coleção  
`action process_items (lista_usuarios)`  
    `# Iteração simples`  
    `for user lista_usuarios`  
        `echo! user.nome`  
      
    `# Iteração com destruturação (Tuplas/Mapas)`  
    `let mapa_pontos get_scores!`  
    `for (nome, pontos) mapa_pontos`  
        `echo! "#{nome}: #{pontos}"`

    `# Iteração numérica (Range)`  
    `for i (range 0 10)`  
        `echo! "Índice: #{i}"`

### **Tratamento de Erros**

1. **Erros Recuperáveis:** Retornam Result::(Ok, Err). O chamador deve tratar o erro.  
2. **Erros Irrecuperáveis:** Utilizam panic\!. Encerram a Action atual imediatamente.  
   * Ex: Divisão por zero em literais, estouro de memória, violação de invariante de tipo em runtime.

`# Exemplo de tratamento em Action`  
`action process_input`  
    `let raw input!("Digite número:")`  
      
    `# Tratamento explícito de erro de conversão via Pattern Matching`  
    `match (parse_int raw)`  
        `Ok val: echo! (+ val 1)`  
        `Err e:  echo! "Input inválido"`

### **Concorrência (Modelo CSP)**

A concorrência é exclusiva do domínio das **Actions**. A linguagem adota o modelo CSP (Communicating Sequential Processes), onde processos isolados (Green Threads) não compartilham memória, mas comunicam-se estritamente através de canais.

#### **1\. Primitivas de Canal**

Canais são tipados e direcionais na criação (Sender/Receiver).

* **channel\! :: T \-\> (Sender::T, Receiver::T)**: Cria um canal síncrono (Rendezvous). Garante entrega. O envio bloqueia até que haja um receptor, e vice-versa.  
* **queue\! :: Int T \-\> (Sender::T, Receiver::T)**: Cria um canal assíncrono com buffer de tamanho fixo. Garante entrega enquanto houver espaço. O envio bloqueia se o buffer estiver cheio (Backpressure).  
* **broadcast\! :: T \-\> (Sender::T, Subscribe::(Int \-\> Receiver::T))**: Cria um canal de difusão.  
  * **Comportamento:** O Sender **nunca** bloqueia.  
  * **Inscrição:** A função Subscribe(tamanho\_buffer) cria um receiver com buffer circular independente.  
  * **Garantias:** Se o buffer do receiver encher, as mensagens mais antigas são sobrescritas (Drop-Oldest). A responsabilidade de processar rápido o suficiente é inteiramente do consumidor.  
* **spawn\! :: Action \-\> Void**: Executa uma Action em uma nova *green thread* concorrente.

#### **2\. Operações de Comunicação**

Seguem a sintaxe prefixa padrão.

* **send\! :: Sender::T T \-\> Void**: Envia um valor para o canal. Bloqueia dependendo do tipo do canal.  
  * Sintaxe: send\! canal valor  
* **recv\! :: Receiver::T \-\> T**: Bloqueia a execução até receber um valor do canal.  
  * Sintaxe: recv\! canal  
* **try\_recv\! :: Receiver::T \-\> Optional::T**: Não bloqueia; retorna Optional val se houver dados, ou None.

#### **3\. Select (Escolha Não-Determinística)**

A estrutura select\! é o mecanismo de controle de fluxo fundamental para concorrência. Ela permite que uma *green thread* aguarde múltiplos eventos simultaneamente, reagindo ao **primeiro** que estiver disponível.  
**Comportamento do Runtime:**

1. **Avaliação:** Todas as operações de canal (recv\!, send\!) listadas nos case são avaliadas para verificar prontidão.  
2. **Bloqueio:** Se nenhum canal estiver pronto, a execução é suspensa até que um deles desbloqueie ou o timeout expire.  
3. **Aleatoriedade Justa (Fairness):** Se múltiplos canais estiverem prontos simultaneamente, o runtime escolhe **aleatoriamente** qual case executar. Isso impede que um canal muito ativo cause *starvation* (inanição) nos outros.

`select!`  
    `# Tenta receber de A. Se houver dados, executa o bloco.`  
    `case (recv! canal_a) -> valor:`  
        `echo! "Prioridade A: #{valor}"`  
      
    `# Tenta receber de B.`  
    `case (recv! canal_b) -> valor:`  
        `echo! "Prioridade B: #{valor}"`  
      
    `# Tenta enviar para C. Só executa se C estiver pronto para receber.`  
    `case (send! canal_c 100):`  
        `echo! "Enviado 100 para C"`  
      
    `# Opcional: Executa se nada acontecer em N ms.`  
    `timeout! 5000:`  
        `echo! "Timeout: Ninguém respondeu."`

## **6\. Estratégia de Compilação e Otimização**

1. **Análise de Dead Code:** Funções não alcançáveis a partir das Actions raiz são descartadas.  
2. **Strictness Analysis:** O compilador força a avaliação estrita de operações aritméticas dentro de funções para evitar a criação de thunks.  
3. **Tail Call Optimization (TCO) Obrigatória:** Como as funções operam em modo estrito e não possuem laços de repetição imperativos (for/while), o compilador deve garantir a otimização de chamadas recursivas de cauda. Isso transforma recursão em iteração (assembly jump) para evitar Stack Overflow em processamentos longos.

## **7\. Diretivas de Compilação e Runtime**

As diretivas são anotações que alteram o comportamento não-funcional do código (performance, agendamento, armazenamento), sem alterar sua semântica lógica. Elas são prefixadas por @.

### **Controle de Concorrência (@parallel)**

Por padrão, spawn\! cria uma *Green Thread* (processo leve gerenciado pela VM). A diretiva @parallel instrui o runtime a utilizar um **Processo de Sistema Operacional** isolado.

* **Uso:** Para tarefas intensivas de CPU (CPU-bound) que travariam o agendador da VM, ou para isolamento total de falhas.  
* **Custo:** A comunicação via canais com ações @parallel envolve serialização de dados (IPC), sendo mais custosa que a comunicação entre Green Threads.  
* **Restrição de Serialização:** Os dados enviados para canais conectados a processos @parallel devem ser **Serializáveis**.  
  * **Funções:** São permitidas (pois não carregam estado/closure).  
  * **Recursos Vinculados (System Handles):** Referências a arquivos abertos, sockets ou conexões de banco de dados. Embora sejam tratados sintaticamente como dados, eles representam capacidades de I/O atreladas ao processo local do SO e não podem ser serializados.

`@parallel`  
`action processamento_pesado (input_ch output_ch)`  
    `# Roda em processo separado do SO`  
    `let dados recv! input_ch`  
    `# ... computação intensa ...`  
    `send! output_ch resultado`

### **Estratégia de Cache (@cache\_strategy)**

Aplicável apenas a **Funções** (puras). Instruí o compilador a gerar automaticamente um *wrapper* de memoização para a função. Como as funções são puras, o retorno é garantido ser o mesmo para os mesmos argumentos.  
**Parâmetros:**

* size: Número máximo de entradas no cache (política de substituição LRU padrão).  
* ttl: Tempo de vida (Time-To-Live) da entrada em milissegundos.

`@cache_strategy{size: 1000, ttl: 60000}`  
`fibonacci :: Int -> Int`  
`λ (n) ...`

## **8\. Sistema de Dados e Estruturas Padrão**

Abaixo estão as estruturas fundamentais da biblioteca padrão. Todas são **imutáveis** por padrão (exceto dentro de Actions onde referências podem ser reatribuídas).

### **Tipos Primitivos**

Os blocos de construção atômicos da linguagem.

* **Int / Float:** Numéricos com precisão definida pela plataforma (64-bit padrão).  
* **Byte:** Inteiro sem sinal de 8-bit (0-255), essencial para I/O binário.  
* **Bool:** Lógico (true, false).  
* **Text:** String UTF-8 imutável.  
* **Unit:** Representado por (), indica ausência de valor (similar a void).

### **Estruturas Algébricas e Coleções**

* **Tuple:** Sequência heterogênea de tamanho fixo. (1, "a", true).  
* **List::T:** Sequência homogênea encadeada. Otimizada para recursão (head/tail). Utiliza **Contagem de Referência (RC)** internamente para permitir compartilhamento estrutural.  
* **Map::K::V:** Dicionário chave-valor (Hash Map imutável).  
* **Set::T:** Conjunto de valores únicos.

### **Arrays Numéricos e Matrizes (NDArray)**

Diferente de listas, estas estruturas garantem **memória contígua** e alinhamento para uso de instruções vetoriais (SIMD). São essenciais para a performance numérica (inspiração em Julia/NumPy).

* **Vector::T:** Array 1D. Crescimento dinâmico (como Vec em Rust), mas imutável na visão do usuário.  
* **Matrix::T:** Array 2D. Implementado como um bloco único de memória (*Row-Major* por padrão).  
  * Suporta operações algébricas diretas via interfaces NUM (+, \* matricial).

`# Construção via literal (Notação de ponto-e-vírgula para quebra de linha)`  
`let mat_a [1 2 ; 3 4]  # Matriz 2x2`

`# Acesso (Indexação baseada em 0)`  
`let val (get mat_a 0 1) # Linha 0, Coluna 1 -> 2`

### **Uniões e Variantes (Tagged Unions)**

A estrutura data permite definir **Tipos Soma** (Sum Types), onde um valor pertence a exatamente uma variante possível de um conjunto fechado. Isso unifica Enums, Unions e tipos opcionais em um único conceito.  
**Sintaxe:** data Nome (Variante1::Tipo, Variante2, ...)

#### **Exemplo: Union Heterogênea**

`# Uma forma pode ser um Círculo (tem raio) OU Retângulo (tem largura e altura)`  
`data Shape`  
    `Circle::Float`  
    `Rectangle::(Float, Float)`

`λ (area s)`  
    `match s`  
    `Circle r:        * PI (* r r)`  
    `Rectangle (w h): * w h`

#### **Optional e Result (Standard Library)**

Estes tipos fundamentais são apenas Unions padronizadas, sem mágica de compilador.

* **Optional::T** Representa a presença (Optional) ou ausência (None) de um valor. Substitui o conceito de null.  
  `data Optional::T (Optional::T, None)`

* **Result::T::E** Representa sucesso (Ok) ou falha (Err).  
  `data Result::T::E (Ok::T, Err::E)`

### **Exaustividade do Match**

O Pattern Matching sobre uma Union deve ser **exaustivo**. O compilador rejeitará o código se alguma variante não for tratada ou se não houver um otherwise.

## **9\. Módulos e Encapsulamento**

O sistema de módulos foi desenhado para garantir encapsulamento robusto e prevenir o "Inferno de Dependências" (onde o comportamento de um tipo muda dependendo de qual arquivo foi importado).

### **Estrutura Básica**

* **Unidade:** Cada arquivo fonte (.kata) é um módulo.  
* **Visibilidade:** Por padrão, todas as definições (data, function, action) são **privadas**.  
* **Exportação:** A palavra-chave export torna o símbolo acessível a outros módulos.  
* **Execução:** Imports são declarativos. Apenas o arquivo de entrada ("main") executa Actions no escopo global. Módulos importados apenas fornecem definições.

`# Arquivo: math/geometry.kata`

`# Privado (detalhe de implementação)`  
`let PI 3.14159`

`# Público`  
`export data Point(x y)`

`export area :: Point -> Float`  
`λ (p) * PI (* p.x p.x) # Exemplo simplificado`

### **Importação**

Traz definições de outros módulos para o escopo atual.  
`import math/geometry          # Acesso qualificado: geometry.Point`  
`from math/geometry import Point # Acesso direto: Point`

### **A Regra de Coerência (Orphan Rule)**

Para garantir a estabilidade do ecossistema (inspirado em Rust), a linguagem proíbe **Instâncias Órfãs**.  
**Regra:** Para implementar uma Interface para um Tipo, **pelo menos um dos dois** deve ter sido definido no módulo atual.  
Isso impede que uma biblioteca de terceiros quebre seu código silenciosamente ao adicionar uma implementação conflitante no futuro.

| Origem do Tipo | Origem da Interface | Implementação Permitida? |
| :---- | :---- | :---- |
| **Local** | **Local** | ✅ Sim |
| **Local** | Externa | ✅ Sim |
| Externa | **Local** | ✅ Sim |
| Externa | Externa | ❌ **NÃO** (Violação de Coerência) |

### **Extensão de Tipos Externos (Newtype Pattern)**

Se for necessário implementar uma Interface externa para um Tipo externo (o caso proibido acima), deve-se utilizar o padrão **Newtype**.  
Um Newtype é um tipo definido localmente que encapsula o tipo externo.

* **Custo Zero:** O compilador remove a camada extra em tempo de execução; o Newtype tem a mesma representação em memória que o tipo original.  
* **Segurança:** Para o sistema de tipos, eles são distintos, o que satisfaz a regra de coerência.

`import libs/json (Jsonizable) # Interface Externa`  
`import libs/math (Vector)     # Tipo Externo`

`# Erro de Compilação: Violação de Coerência`  
`# implement Jsonizable for Vector ...`

`# Solução: Definir um Newtype local`  
`# 'data' com um único campo é tratado como Newtype`  
`data MyVector(v::Vector)`

`# Permitido: MyVector é definido neste módulo`  
`implement Jsonizable for MyVector`  
`λ (to_json self) ...`

## **10\. Modelo de Memória (Sem GC)**

A linguagem não possui Garbage Collector (GC). O gerenciamento é determinístico, baseado em posse (Ownership) e contagem de referências localizada.

### **Semântica de Movimento (Move Semantics)**

Para tipos complexos (Vetores, Strings, Structs), a atribuição ou passagem de argumentos transfere a posse.

* O escopo dono é responsável por liberar a memória (Drop) quando a variável sai de escopo.  
* Cópias profundas (Clone) devem ser explícitas.

### **Tipos Copiáveis (Copy)**

Tipos primitivos (Int, Float, Bool) são baratos de copiar. Eles não sofrem Move, são duplicados implicitamente.

### **Contagem de Referência (RC)**

Para estruturas de dados persistentes que exigem compartilhamento estrutural (como List::T encadeada), a linguagem utiliza Contagem de Referência (Reference Counting) interna. Isso permite que múltiplas listas compartilhem a mesma cauda, sendo liberadas apenas quando o último "dono" desaparecer.

## **11\. Notas de Design (Originais)**

Esta seção preserva as decisões filosóficas originais que guiaram a criação da sintaxe e estrutura da Kata-Lang.

### **Blocos x Pattern Matching x Guards**

Blocos de código imperativos tradicionais, por serem extensos e aninhados, são mais propensos a erros do que definições declarativas. Por isso, adotou-se o esquema de **Pattern Matching** inspirado em Haskell: o código tenta encaixar o input em padrões definidos e, dessa forma, escolhe o "corpo" da função a ser executado.  
Embora possa assemelhar-se a um if-else encadeado ou switch-case à primeira vista, neste contexto a estratégia é vital para segregar comportamentos do código estritamente de acordo com o formato do input, diminuindo a superfície de bugs lógicos.  
Ainda mais próximo do conceito de switch-case são os **Guards**. Eles diferem do Pattern Matching pois não casam padrões estruturais, mas sim testam condicionais booleanas para escolher a linha a ser executada.

* **Distinção Importante:** Guards realizam computação (testes lógicos complexos), enquanto Pattern Matching verifica estrutura e tipos (são mais "simples" e diretos). Embora possam parecer redundantes juntos, eles cobrem espectros diferentes da lógica de controle de fluxo.