# **Kata-Lang**

## **Especificação Técnica (Rev v3.0)**

## **1\. Filosofia e Arquitetura: O Modelo "Delayed-Strict"**

A Kata-Lang opera sobre uma segregação rígida de responsabilidades para garantir segurança, previsibilidade e performance. O modelo adota **Execução Atrasada (Delayed Execution)** para a construção da topologia do programa e **Avaliação Estrita (Strict Evaluation)** para a execução de primitivas.

### **A Tríade**

1. **Dados (Data):**  
   * Estruturas de informação imutáveis e serializáveis (Value Objects).  
   * São passivos e não contêm comportamento oculto.  
2. **Cálculos (Functions):**  
   * Definidores de topologia. Uma função **não computa** dados pesados; ela descreve um nó em um Grafo Acíclico Dirigido (DAG).  
   * A aplicação de uma função é uma operação de custo O(1) que apenas anexa uma instrução ao plano de execução.  
   * São puras, determinísticas e transparentes referencialmente.  
3. **Ações (Actions):**  
   * O gatilho de materialização. É o ponto onde o Grafo de Execução é compilado e os dados começam a fluir.  
   * Responsáveis por efeitos colaterais (I/O), aonde mutabilidade local é permitida e onde é definida a orquestração de concorrência.

### **Modelo de Execução: Stream-based Strict Evaluation**

Ao contrário de linguagens puramente preguiçosas (como Haskell) que podem acumular *Thunks* (promessas de cálculo) na memória causando *Space Leaks*, a Kata-Lang utiliza um modelo de **Pipeline de Fluxo**:

1. Fase de Definição (Delayed):  
   Quando o código let y \= map (+ 1\) lista é avaliado dentro de uma função, nada acontece com a lista. O runtime apenas aloca um pequeno descritor (nó do grafo) indicando que, quando os dados passarem por ali, deverão ser incrementados.  
   * *Memória:* Zero duplicação de dados, zero *thunks* de dados.  
2. Fase de Materialização (Action Trigger):  
   Quando uma Action solicita os dados (ex: write\! y ou um loop for), o pipeline é ativado. "Drivers" (Iteradores) são instanciados para puxar os dados da fonte através do grafo.  
3. Fase de Processamento (Strict Primitive):  
   O dado flui item a item (ou em chunks vetorizados) pelo grafo.  
   Crítico: Quando um dado atinge um nó de operação primitiva (soma, lógica, comparação), a operação é executada estritamente e imediatamente na CPU/Registradores.  
   * *Exemplo:* Em \+ x 1, não se cria um objeto Soma(x, 1). O valor é calculado, o registrador é atualizado e o valor cru passa para o próximo passo do pipeline.

## **2\. Sintaxe e Notação**

A linguagem utiliza **notação prefixada (Polish Notation)** obrigatória. Essa escolha elimina a necessidade de regras complexas de precedência de operadores e parênteses excessivas para desambiguação.

### **Regra de Ouro do Parser**

O espaço em branco é o delimitador funcional estrito.

* \+ 1 1 é uma função de soma aplicada a dois argumentos.  
* \+1 1 é interpretado como identificador de função (+1) aplicado a 1\.  
* \- 10 é uma subtração (esperando argumento).  
* \-10 é o literal numérico negativo.

### **Convenções de Identificadores**

* **Interfaces (ALL\_CAPS):** Contratos universais (ex: NUM, ORD).  
* **Tipos (CamelCase):** Estruturas de dados (ex: Int, Vector, T).  
* **Funções e Variáveis (snake\_case):** Valores e computações.

### **Estrutura de Funções**

Uma função nomeada é um template para um nó do grafo de execução.

\# Assinatura: Recebe 3 inteiros, retorna Texto

fizzbuzz :: Int Int Int \-\> Text  
λ (fizz\_num buzz\_num x)  
    both: "FizzBuzz"  
    fizz: "Fizz"  
    buzz: "Buzz"  
    otherwise: str x  
    with fizz as \= (mod x fizz\_num) 0  
         buzz as \= (mod x buzz\_num) 0  
         both as (and fizz buzz)

### **Aplicação Parcial e Piping**

A aplicação parcial deve ser explícita com ? para evitar erros de aridade em um sistema sem parênteses aninhados.

* **Currying Explícito:** let soma\_10 (soma 10 ?)  
* **Piping (|\>):** Facilita a leitura linear do fluxo de dados.  
  * dados |\> map f1 ? |\> filter f2 ?


## **3\. Sistema de Tipos e Verificação**

Tipagem forte, estática (com inferência) e suporte a **Tipo-Dependência (Refinement Types)**.

### **Tipo-Dependência e Fronteiras**

A validação de tipos refinados (ex: Inteiro positivo) ocorre nas bordas do sistema (Actions, Literais ou I/O).  
Dentro do pipeline de funções (o "miolo" do grafo), o compilador assume que os invariantes são respeitados, eliminando checagens redundantes em runtime e permitindo otimizações agressivas.  
\# Definição de um tipo refinado  
data PositiveInt as |Int, \> ? 0|

\# Uso em Generics com Restrição  
max :: T T \-\> T  
λ (x y)  
    \> x y: x  
    otherwise: y  
with T as |T implements ORD|

### **Generics e Monomorfização**

O compilador utiliza **Monomorfização**. Para cada tipo concreto que utiliza uma função genérica, uma versão especializada em código de máquina é gerada. Isso garante que abstrações de alto nível (como map ou filter) tenham custo zero em tempo de execução (*Zero-Cost Abstractions*).

## **4\. Interfaces e Polimorfismo**

Interfaces definem contratos de comportamento. A linguagem segue a **Regra de Coerência (Orphan Rule)**: Para implementar uma Interface para um Tipo, pelo menos um dos dois deve ser definido no módulo local.

### **Sobrecarga de Operadores (Static Resolution)**

Operadores (+, \-, \*) são funções normais definidas em interfaces (NUM). A resolução da sobrecarga acontece estaticamente durante a construção do DAG.  
\# Vetor 2D  
data Vec2 (x, y) implements NUM

\# Implementação da soma para Vec2  
\# Define como o nó "Soma" se comporta quando dados Vec2 passam por ele  
\+ :: Vec2 Vec2 \-\> Vec2  
λ (a b)  
    Vec2 (+ a.x b.x) (+ a.y b.y)

## **5\. Actions: O Motor de Execução**

As Actions são o domínio da **Mutabilidade Local** e do **I/O**. Elas orquestram a execução.

### 

### **Controle de Fluxo Imperativo**

Actions podem instanciar "Drivers" (Iteradores) que puxam dados dos grafos funcionais definidos anteriormente.  
action processar\_logs (caminho)  
    \# 1\. Definição do Grafo (Delayed \- Custo Zero)  
    \# Nada é lido do disco aqui. Apenas se monta o plano.  
    let fluxo\_bruto  read\_file\_stream\! caminho  
    let fluxo\_limpo  fluxo\_bruto |\> map parse\_log ? |\> filter is\_error ?

    \# 2\. Materialização (Strict Execution)  
    \# O 'for' atua como o Driver que consome o fluxo item a item.  
    for erro fluxo\_limpo  
        echo\! "Erro Crítico: \#{erro.msg}"

### **Concorrência (CSP)**

A concorrência ocorre estritamente via troca de mensagens, sem memória compartilhada entre processos.

* **channel\!:** Canal síncrono (Rendezvous).  
* **queue\!(size):** Canal assíncrono com buffer (Backpressure).  
* **broadcast\!:** Canal de difusão (1 \-\> N).  
* **Isolamento:** Cada Action (seja *Green Thread* ou Processo OS) possui seu próprio heap. Não há GC global "Stop-the-world". A limpeza é feita via *Ownership* local quando a Action termina.

## 

## **6\. Estratégia de Compilação e Otimização**

A arquitetura *Delayed-Strict* viabiliza otimizações profundas.

### **Stream Fusion (Fusão de Fluxo)**

Como as funções definem uma topologia antes de executar, o compilador identifica cadeias de produtores/consumidores e as funde.

* *Código:* lista |\> map (+1) ? |\> map (\*2) ?  
* *Compilação:* O compilador não gera dois loops com um buffer intermediário. Ele gera **um único loop** em código de baixo nível que carrega o valor em registrador, incrementa, multiplica e emite o resultado.  
* *Benefício:* Eliminação total de alocação de memória temporária para transformações intermediárias.

### **Strictness Analysis e TCO**

Como a avaliação de primitivas é estrita, o compilador pode garantir a segurança da pilha.

* **Tail Call Optimization (TCO):** É mandatória. Recursões de cauda (f chama f como última instrução) são compiladas como JUMP (iteração), não como CALL (pilha).  
* Se uma função recursiva não puder ser otimizada (não for de cauda), o compilador deve emitir um aviso, pois o modelo estrito não suporta profundidade de pilha infinita.

### **Diretivas de Runtime**

* @parallel: Quebra o grafo de execução em sub-grafos conectados por canais IPC, distribuindo o processamento.  
* @cache\_strategy: Insere nós de memoização (Hash Table) no grafo para evitar recomputação de sub-árvores puras.

## **7\. Estruturas de Dados (Standard Library)**

Todas as estruturas são imutáveis por padrão.

* **Primitivos:** Int, Float, Byte, Bool, Unit.  
* **Coleções (Persistent):**  
  * List::T: Encadeada, otimizada para *head/tail*.  
  * Map::K::V, Set::T.  
* **Arrays Numéricos (Contiguous Memory):**  
  * Vector::T e Matrix::T. Otimizados para SIMD e acesso linear.  
* **Uniões:**  
  * Optional::T (Some/None).  
  * Result::T::E (Ok/Err).

## **8\. Módulos e Encapsulamento**

* **Unidade:** Arquivo .kata.  
* **Visibilidade:** Privada por padrão. export expõe símbolos.  
* **Imports:** Apenas disponibilizam definições. Não executam código lateral.

## **9\. Tratamento de Erros**

* **Funções (Grafo):** Erros são valores (Result) que fluem pelo pipeline. O fluxo não é interrompido magicamente; o dado "Erro" viaja até ser tratado.  
* **Actions (Runtime):** Podem usar panic\! para estados irrecuperáveis (bugs, falta de memória) ou tratar Result via *Pattern Matching* ou unwrap.

\# Exemplo de fluxo seguro  
safe\_div :: NUM NUM \-\> Result::NUM  
λ (x y)  
    \= y 0: Err "Divisão por zero"  
    otherwise: Ok (/ x y)  
