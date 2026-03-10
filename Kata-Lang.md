# Inspirações:

- Haskell: sintaxe fora de série, sistema de tipos infernal  
- Python: sintaxe linda, praticidade excepcional  
- Shellscript: A capacidade de encadear processos é excelente  
- Rust, Elixir…  
- Apache Spark: Forma de pensar  
- julia: A minha ideia de polimorfismo é muito próxima do multiple dispatch dessa linguagem

# TL;DR:

\- Linguagem que separe o código em 3 "domínios":  
\- dados (data), que são os os dados processados  
\- cálculos, que são as funções (puras)  
\- ações (actions), que são código com efeito colateral (impuro)  
\- As funções são lazy, actions são eager  
\- Valores dentro de funções são \*\*constantes\*\*, valores dentro de actions \*\*variáveis\*\*.  
\- Uma otimização possível é considerar apenas o conteúdo de actions ao compilar  
\- Ao executar um programa escrito nessa linguagem, somente as actions chamadas são executadas.  
\- Notação pré-fixada  
\- Sistema de tipos forte, com inferência, tipo-dependência, interfaces e Generics (parametric polymorphism).

# Notas:

#### Memória durante a execução:

           Com valores imutáveis, podemos saber em tempo de compilação os tipos

#### Laziness x Recursão de Cauda:

A vantagem da recursão de cauda é evitar estouro de pilha. Com comportamento lazy, esse risco é fortemente mitigado, além de simplificar o esforço de implementação do código.

#### Blocos x Pattern Matching x Guards:

	Blocos de código, por serem maiores, são mais propensos a erros que definições declarativas. Por isso, o esquema de Pattern Match de Haskell é muito interessante, o código tenta encaixar o input em padrões e dessa forma escolher o “corpo” da função a ser usado.  
	Me incomodou um pouco no início a semelhança com if-else encadeado, ou com switch-case, mas nesse contexto faz bastante sentido e parece muito útil para segregar comportamentos do código de acordo com o input e, dessa forma diminuir a possibilidade de erros no código.  
	Ainda mais parecido com switch-case são os Guards, que não realizam Pattern-Match, mas condicionais, para escolher a linha a ser executada.  
	Guards e Pattern-Matching podem soar redundantes, mas Guards realizam computação (servem para isso aliás), enquanto Pattern-Matching não fazem o mesmo, são mais “simples”.

#### Espaços e Notação prefixa “bizarra”:

	A notação prefixa é chocante à primeira vista, afinal toda educação em matemática usa a notação infixa, e toda linguagem de programação também. Certo?  
	Mais ou menos… estamos acostumados a notação infixa, mas quando aprendemos funções fora da aritmética, usamos notação prefixa.  
	Na definição “*f(x) \= x+1*”, ao usar a função *f* para o valor y, escreve-se “*f(y)*”, e na eventualidade de existir uma função *g*, para qual queremos aplicar no valor *f(y)*, escrevemos “*g(f(y))*”, isso já é notação prefixa.  
	E nós ainda misturamos infixa e prefixa, por exemplo, “*f(g(x)) \+ x \+ g(f(x))*”. Isso é mais confuso do que pensamos. Só não percebemos por que nos habituamos cedo e praticamos à exaustão.  
	Soma, divisão e multiplicação são funções . Não são particularmente diferentes de quaisquer outras. Por que essa diferença de tratamento? Ou, dado que já estamos acostumados com esse modo de fazer, talvez a pergunta devesse ser “por que não?”.  
	Bem, o primeiro motivo é a implementação de precedência. Notação infixa gera a necessidade de se estabelecer regras de precedência para definir qual função deve ser aplicada primeiro em relação às demais, e também torna essencial o uso de organizadores, como parênteses, chaves, colchetes, vírgulas e etc, tanto para humanos lendo, como para à máquina executando o código.   
A notação prefixa não tem esse problema, os organizadores ainda são necessários para o nosso entendimento e conforto (dependendo da complexidade da expressão escrita), porém para a máquina executando o código, eles são irrelevantes. E isso também torna a implementação da linguagem mais simples, já que não é necessário tratar os operadores aritméticos de maneira especial.   
Além de tornar a escrita visualmente menos densa quando quebramos o código em pedaços simples o suficiente, dado que deixa de ser obrigatório o uso dos organizadores. Isto também pode ser verdade para a notação infixa, mas comparando o melhor caso em cada notação, a prefixa é mais legível.  
A notação prefixa também cria a necessidade de um uso correto de espaços entre as funções em argumentos: “1+1” é perfeitamente válido e compreensível, tanto quanto “1 \+ 1”. Na notação prefixa “+ 1 1” é válida, porém “+1 1” não seria, e o motivo se torna evidente quando se substitui o nome da função de soma (“+”) por “somar”: “`+ 1 1"` se torna “`somar 1 1`" e eliminar o espaço entre o operador e o primeiro argumento (`somar1 1`)  causa a referência a uma função de nome diferente, “somar1” em vez de “somar”, que pode nem mesmo ter sido definida.   
Isso deixa a sintaxe de números com os símbolos de \+ e \- disponível,  sem causar confusão na leitura ou complexidade na implementação; “-10”, sem espaço, é “10 negativo”, enquanto “- 10” é uma subtração à espera do segundo argumento.  
Em conjunto com a estratégia de pattern-matching, eu *espero* que essa notação acabe deixando o código mais limpo, e mais fácil de se implementar e manter.

# Exemplos e Explicações:

#### Lambdas:

	Funções anônimas, que recebem valores como parâmetros, e tem um corpo de 1 linha ou um bloco de Guards.

Exemplo:

`λ (x) + 1 x`  
`lambda (x) + 1 x`

As duas linhas são equivalentes, o símbolo λ e a palavra lambda têm o mesmo valor.  
O Lambda descrito espera um input “x” e aplica uma soma (“+”) com o valor 1\.

`λ (x y)`   
`> x 0: + y x`  
`= y 0: x`  
`otherwise: y`

O Lambda anterior é composto por Guards, cada linha indentada é um Guard, onde a expressão antes do “:” é testada, e caso seja verdadeira a linha é executada. No caso, se x for maior que zero, x e y serão somados. Se não, a igualdade entre y e 0 é testada, em caso verdadeiro a função retorna x, em qualquer outra situação, a função retorna y.

`λ (x y)`   
`> a 0: + y a`  
`= b 0: + x b`  
`otherwise: y`  
`with a as - x 1`  
     `b as * y x`

Este exemplo inclui um “with”, usado para evitar repetição de operações.

#### Sintaxe de funções (nomeadas):

Moldura/decorator da função (opcionais)  
Assinatura (nome \+ ((tipos de entrada e saída) opcional))  
“Lambdas” (emolduradas ou não)

Exemplo: Função “FizzBuzz”

`1. fizzbuzz`  
`2. λ (fizz_num buzz_num x)`  
`3.	both: "FizzBuzz"`  
`4.	fizz: "Fizz"`  
`5.	buzz: "Buzz"`  
`6.	otherwise:  str x`  
`7.	with fizz as = (mod x fizz_num) 0`  
`8.		buzz as = (mod x buzz_num) 0`   
`9.		both as (and fizz buzz)`  
`10.`

Essa função chama-se “fizzbuzz”  (linha 1).  
Ela é composta de 1 lambda (linha 2\) que espera 3 argumentos: “fizz\_num”, “buzz\_num” e “x”.  
O lambda em questão tem 4 Guards (linhas 3, 4, 5 e 6\) e a cláusula “with” (linha 7\) definindo os valores “fizz”, “buzz” e “both” que são utilizados nos Guards.  
Na linha 4, o Guard será usado se “both” for verdadeiro, então a execução da função será encerrada, retornando “FizzBuzz”.  
Quando “both” for falso, a função testará o Guard da linha 5 (valor de “fizz”), caso seja verdadeiro a função será encerrada retornando “Fizz”. E assim por diante.  
Se todos os Guards forem falsos, o Guard “otherwise” definirá o retorno.

Exemplo 2: Múltiplos lambdas na mesma função, e Pattern-Matching

`1. fibonacci`  
`2. λ (2 x y) + x y`  
`3. λ (n x y) fibonacci (- n 1, y, + x y) # vírgula facilita leitura`  
`4.							   # quando há funções aplicadas`  
`5. fibonacci 6 1 1 	# resultado 8`

A primeira linha determina o nome da função, a linha seguinte será usada sempre que o primeiro argumento for igual a 2, e resultará na soma dos outros 2 argumentos, sejam quais forem. Quando o primeiro argumento não é igual a 2, o primeiro lambda é ignorado. O segundo lambda aceita qualquer padrão, então ele será usado nesse caso, e a sua definição é uma recursão operando os argumentos.

Exemplo 3: Fatorial

`1. fatorial`  
`2. λ (1) 1`   
`3. λ (x) * x (fatorial (- x 1))`  
`4.`

Limites das funções:

- Funções não podem executar actions  
- Quando funções possivelmente devolvem None, o tipo é o enum Option que contém o None ou o valor resultado

#### Tipos, Tipagem e Tipo-Dependência

A tipagem deve ser forte, auto detectado por padrão, estática opcionalmente.   
Ou seja, ao declarar uma variável não é necessário determinar seu tipo, ele deve ser determinado dinamicamente de acordo com seu uso no código. Se o programador quiser, ele pode determinar o tipo da variável ao declará-la e, neste caso, a variável aceitará \*apenas\* valores que sejam do tipo especificado.

Tipos Base:

- NUM (Na verdade, esse é uma Interface para Int, Float, Decimal, Fract e Complex e qualquer coisa que for, em essência, um número )  
- Text  
- Byte  
- Bool  
- Lambda (funções são desse tipo)  
- Interface (contrato que outros tipos atendem)  
- Action (tipo referente à actions, as “funções impuras”)

Exemplo 4:  
`# atribuição de um valor`  
`let x 1`  
`let b true`

`# atribuição de um valor, especificando seu tipo`  
`let x::Int 1`  
`let b::Bool true`

Funções também tem tipo dinamicamente determinado, a menos que o programador decida explicitar. Para isso, à assinatura se segue “::”, depois a lista de tipos dos argumentos da função. Depois disso se segue “-\>” e o tipo do retorno.

Exemplo 5:  
`soma_tres :: NUM NUM NUM -> NUM`  
`λ (x y z) + (+ x y) z`  
\* O trecho destacado em itálico não precisa estar entre parênteses, mas usá-las facilita o entendimento do código nesse caso. 

Tipo-Dependência é uma forma de restringir quais valores uma função pode receber ou retornar, e que uma variável pode assumir:

Exemplo 6:  
`# x é um inteiro que deve ser maior que zero e menor ou igual a 3`  
`# e nesse caso, passa a existir com o valor 2`  
`let x::|Int, > ? 0, <= ? 3| 2`

Caso essa variável seja modificada mais a frente no código, ela só poderá aceitar valores dentro das condições previstas na sua criação, que no exemplo são as expressões entre barras verticais.

Casos de  violação de restrição são identificados em tempo de compilação.

Da mesma forma, uma função pode ser criada com argumentos e/ou retorno tipo-dependentes.

Exemplo 7: Raiz Quadrada de Newton

`1. sqrt_newton :: |Float, >= ? 0| Float Float -> Float`  
`2. λ (0 _ _) 0`  
`3. λ (n x eps)`  
`4. 	<= (- x nx) eps: nx`  
`5. 	otherwise: sqrt_newton (n nx eps)`  
`6. 	with nx as / (+ x (/ n x)) 2`  
`7.` 

Nesse trecho se define a função “sqrt\_newton”, a mesma espera como primeiro argumento um Float maior ou igual a zero, e outros dois valores quaisquer do tipo Float.

Polimorfismo ocorre nas funções nomeadas dado que uma função nomeada nada mais é que uma coleção de funções anônimas de mesma *aridade* (quantidade de argumentos) e mesmo tipo de retorno, onde a função anônima a ser utilizada será a primeira declarada onde ocorrer sucesso no pattern-matching.

Um segundo nível de polimorfismo é possível considerando funções de mesmo nome que retornam tipos diferentes, mantendo-se a aridade. Apesar disso, não recomendo o uso extensivo dessa estratégia.

Manter a aridade é necessário para que não ocorra ambiguidade na chamada de funções, especialmente em aplicações sequenciais de funções e currying.

Exemplo 8:

`1. # Uma multiplicação para os Naturais usando recursão`  
`2. mul :: |Int, <= 0 ?| |Int, <= 0 ?| -> Int`   
`3. λ (_ 0) 0`  
`4. λ (0 _) 0`   
`5. λ (x 1) x`   
`6. λ (1 x) x`   
`7. λ (x y) + x mul (x (- y 1))`   
`8.` 

Tipos são definidos pelo programador usando a palavra chave “data” seguida pelo nome do tipo, a sequência de dados que irá o compor:

Exemplo 9:

`1. data Complex (r, j) implements NUM`  
`2. 	let real::|NUM except Complex| r`  
`3. 	let imaginary::|NUM except Complex| j`  
`4.`   
`5. interface NUM implements ORD: =,>=,<=,<,>,+,-,*,/,//,**,mod,int`  
`6.`   
`7. let x::Complex (1 2)`  
`8. let y::Complex (1,3)`  
`9.`   
`10. str (+ x y) 		# resultado: 2+5j`  
`11. x.imaginary   	# resultado: 2`

Neste exemplo, é definido o tipo para números complexos, e a única responsabilidade é “guardar” os valores recebidos no seu construtor, os valores precisam implementar a interface NUM, porém não podem ser Complex.

Exemplo:

`data Par as |Int, = (mod ? 2) 0|`  
`# No código acima não é necessário definir, pois Par é entendido como Int; caso uma soma com número ímpar ocorra,`   
`# o resultado será um Int.`  
`Se uma variável x::Par for modificada (exemplo: let x (+ x 1)) um pânico ocorre.`  
`# A menos que as funções da interface sejam re-implementadas para o tipo Par, especificando o que acontece em cada caso`  
`# Em caso de valores tipos-dependentes serem combinados, o resultado herda as restrições dos constituintes, a menos que a função que realiza essa combinação tenha sido definida para agir diferente.`

#### Interfaces:

        Interface é o modo de criar interoperabilidade entre tipos que pertencem conceitualmente a mesma categoria, além de associar funções a tipos.  
	Para fácil diferenciação entre tipos e interfaces, tipos tem a primeira letra maiuscula, e interfaces são escritas completamente em maiusculas.  
        Quando um tipo implementa uma interface, é esperado que sejam implementadas todas as funções descritas pela interface, de forma que tipos diferentes que implementam a mesma interface possam ser tratados como equivalentes ao usar uma função definida na interface.  
        Por exemplo, números podem ser comparados entre si, e textos podem ser comparados entre si; ambos implementam ORD, porém, números não precisam ser comparáveis a texto, então não é necessário definir funções que estabeleçam comparação entre números e texto; Em caso de tentativa de comparação, um erro é emitido em tempo de compilação acusando a ausência de funções disponíveis para realizar a comparação: *"*No 'function\_name' function available for these arguments types (type list) on the scope.*"*

`interface ORD: (=, >=, <=, <, >)`  
`# Por que não dá erro colocar funções em lista?`  
`# Para serem aplicadas, as funções precisam ser seguidas de um espaço, seguido dos argumentos.`   
`# A vírgula e o fim das parênteses nega isso.`

`# números precisam implementar as funções de ORD`  
`# mas não é necessário explicitar isso ao criar a interface`  
`interface NUM implements ORD:`   
    `(+,-,*,/,//,**,mod,int,float)`

`-> help NUM`  
    `Interface name: NUM`  
    `Implements: ORD`  
    `Required functions:`   
        `(=,>=,<=,<,>,+,-,*,/,//,**,mod,int,float)`  
    `Known types on the scope:`  
        `Int, Float, Complex`

`-> help ORD`  
    `Interface name: ORD`  
    `Required functions:`   
        `(=,>=,<=,<,>)`  
    `Known types on the scope:`  
        `Text, Int, Float, Complex`

`-> > 1 "1"`  
`Compile error: No '>' function available for these arguments types (Text and Int) on the scope.`

`-> = 1.0 1`  
`true`

`-> >= 'a' 'b'`  
`true`

Dois tipos que implementam **diretamente** a mesma interface precisam ser completamente interoperáveis, e cada tipo mais recente precisa garantir a interoperabilidade com os mais antigos.

#### Generics

Para reduzir a duplicação de código, uma estratégia possível é o uso de Generics, que abstrai o tipo da variável da lógica no código.

`max :: T T -> T`  
`λ (x y)`  
    `> x y: x`  
    `otherwise: y`  
`with T as |T implements ORD|`

Esse código, em princípio vale para qualquer tipo *que implementa comparação* , então, para evitar reescrever o mesmo código múltiplas vezes para cada tipo podemos usar Generics; para isso a sintaxe de Generics é uma extensão da de tipo-dependência: `|T implements (Interface1, Interface2, …), Condição1, Condição2, … |` e é definida dentro do escopo em que será usada, usando a cláusula `with`.

Outros exemplos:

`identity :: T -> T`  
`λ (x) x`  
`with T as |T|`

`swap :: (A, B) -> (B, A)`  
`λ (tuple)`  
    `let (a, b) as tuple`  
    `(b, a)`  
`with A as |A|, B as |B|`

`first :: (A, B) -> A`  
`λ (tuple)`  
    `let (a, b) as tuple`  
    `a`  
`with A as |A|, B as |B|`

`sumPositiveList :: List::T -> T`  
`λ (list) ...`  
`with T as |T implements NUM, > ? 0|`  
   
Os tipos genéricos preferencialmente são representados como uma única letra maiuscula, ou CamelCase, assim como os tipos concretos.

#### Currying ou Funções Parciais:

Função parcial se refere a uma função parcialmente aplicada, ou seja, uma função que não recebeu todos os argumentos. A sintaxe para currying é “?” nas posições dos argumentos que faltam:

Exemplo 10:

`1. soma :: NUM, NUM -> NUM`  
`2. λ (x y) (+ x y)`  
`3.`   
`4. # Soma o argumento com o número 4`  
`5. soma_com_quatro`  
`6. (soma ? 4)`  
`7.`   
`8. soma_com_quatro 3   # resultado: 7`

Neste caso, não foi necessário o símbolo lambda na linha 6, por que a função parcial retorna um Lambda.

#### 

#### Piping

Para tornar mais legível a chamada de funções em sequência, podemos usar *piping*.

\# considere a função h que recebe 3 argumentos, a função g que recebe um argumento w e o resultado de h, e a função f que recebe o resultado de g e um argumento k: 

`f (g (h (x y z) w)) k`

Podemos escrever da seguinte forma:

`h x y z |> g w ? |> f ? k`

Onde o resultado da primeira chamada é enviado para a função parcial seguinte, e assim por diante.  
Caso seja necessário, é possível especificar qual o índice desejado do resultado.  
A função h\_2 retorna um a n-upla (a b c), a função f\_2 espera receber os valores c (como primeiro argumento) e a (como segundo argumento) resultantes da função h\_2:

`h_2 x |> f_2 ?:2 ?:0`  

Sem essa notação, cada interrogação vai representar um elemento da saída anterior em ordem.

#### 

#### Composição

Além de simplesmente passar o resultado de uma função como argumento para outra \- e de usar currying \- podemos compor funções (criando novos lambdas) usando o símbolo “**º**”.

`Exemplo:`  
`1. delta :: NUM, NUM, NUM -> NUM`  
`2. λ (a b c)  - (* b b (* * 4 a c))`  
`3.`   
`4. bhaskara :: NUM, NUM, NUM -> NUM`  
`5. λ (a b c)  / (+ -b ( sqrt ° delta (a b c))) (* 2 a)`

`Exemplo:`  
`1. f1`  
`2. λ (x)`  
`3.`   
`4. f2`  
`5. λ (y)`  
`6.`  
`7. f3`  
`8. f1 ° f2 (x)`

`Exemplo:`  
`f1 :: NUM -> (NUM, NUM)  # retorna uma tupla de dois números`  
`f2 :: NUM NUM -> NUM 	# espera dois números, retorna um`  
`f3 :: NUM -> TEXT    	# espera um número, retorna texto`

`# Isso seria válido:`  
`f3 ° f2 ° f1`

`# Porque:`  
`# 1. f1 retorna (NUM, NUM) que corresponde aos dois NUM que f2 espera`  
`# 2. f2 retorna NUM que corresponde ao que f3 espera`  
`# 3. O resultado final é uma função NUM -> TEXT`

#### 

#### Actions

Actions são a parte impura do código, qualquer interação entre o código e o “mundo exterior”.  
Por exemplo, leitura e escrita de arquivos, acesso a banco de dados, print, input e outras interações com o usuário \- além de código que não gerem sempre o mesmo valor dada uma entrada, como um gerador de números aleatórios \- deve acontecer dentro das actions. O ideal é que a maior parte do código esteja escrito na forma de funções e que as actions tenham o tamanho (e quantidade) mínimo necessário para que o código funcione.  
Segregar o código puro e impuro tem como propósito garantir que erros de origem externa não vão afetar a parte “pura” do código e, simultaneamente, tornar mais fácil gerenciar e dar manutenção no código.  
A execução de actions é eager, ou seja, o código de uma action é executado assim que for chamado. As actions **não** podem ser chamadas dentro das funções, apenas o contrário é permitido.

Exemplo 11:

`1. # Hello World`  
`2.`  
`3. action greet`  
`4.	echo!(“Olá MUNDO!”)`  
`5.`  
`6. greet()`  
`7.`

No exemplo 11 é definida a action `greet` que simplesmente chama a action `echo!`, e na linha 6 a action `echo!` é chamada e imediatamente executada, e o console imprimirá o texto “Olá MUNDO\!”.  
A sintaxe das actions: A palavra “action” no início da linha e recuada para o nível atual de indentação seguida do nome da action. Caso a action aceite argumentos, os mesmos devem estar entre parênteses. Nas linhas seguintes o bloco de código deve estar indentado com um nível extra. O código dentro de uma action é executado como em qualquer linguagem imperativa, as funções usadas dentro das actions são tratadas como operações sobre os dados, e os valores dentro do escopo das actions são mutáveis.  
Código dentro das funções não deve estar sujeito a exceções, as exceções devem ser declaradas e tratadas nas actions.

#### Erros, pânicos e validação preemptiva:

	Pânicos são violações fundamentais do estado do programa, devem interromper a execução.  
	Valores inválidos devem (idealmente) ser irrepresentáveis; usando tipo dependência, enums e .  
	Erros devem ser tratados como valor: Result::(Value, Error).

Exemplo:

`1. # Hello Me`  
`2.`  
`3. action greet`  
`4.	let nome input!(‘Qual seu nome?’)`  
`.expect_any(‘Erro ao ler o input.’, exit)`  
`5.		#.expect(ErrorType, ‘Erro ao ler o input.’, exit)`  
`5.	echo(‘Olá #{nome}!’)`  
`6.`  
`7. greet()`  
`8.`  
\[Talvez o ".except" receba uma mensagem de erro e uma função que trata a exceção\]

#### panic\!():

`data NonZero as |NUM, != ? 0|`

`div_a :: NUM NonZero -> NUM`   
`λ (x y) / x y`

`div_b :: NUM NUM -> NUM`   
`λ (x y) = y 0: panic!(“Division by zero.”)`   
`otherwise: / x y`

`action do_div_a`  
	`div_1 1 0  # Panic: Type dependency violation. 0 does not satisfy the constraint '!= ? 0' of type NonZero.`

`action do_div_b`  
	`div_2 1 0  # Panic: Division by zero.`

Concorrência:

A linguagem oferece três primitivas para concorrência, todas usadas exclusivamente dentro de actions:

\`channel\!()\` cria um canal bloqueante, retornando uma tupla (sender, receiver). O envio bloqueia até que haja um receptor pronto, e a recepção bloqueia até haver dados disponíveis. Ideal para sincronização precisa entre actions:

`let (sender, recv) channel!()`  
`>! sender value  # Bloqueia até alguém receber`  
`let data <! recv # Bloqueia até haver dados`

\`queue\!(size)\` cria uma fila com buffer, retornando (sender, receiver). O envio só bloqueia quando o buffer está cheio, e a recepção só bloqueia quando está vazio. O parâmetro size define o tamanho do buffer:

`let (sender, recv) queue!(10)  # Buffer de 10 elementos`  
`>! sender value  # Não bloqueia se houver espaço`  
`let data <! recv # Não bloqueia se houver dados`

\`broadcast\!()\` cria um canal de broadcast, retornando (sender, recv\_f). O sender envia dados para todos os receivers ativos, e recv\_f() é uma função que cria novos receivers. Cada receiver recebe todos os dados enviados após sua criação:

`let (sender, subscribe) broadcast!()`  
`let recv1 subscribe(size=4)  # Cria um receiver com buffer tamanho 4`  
`let recv2 subscribe()  # Cria outro receiver com buffer tamanho 1`  
`>! sender value	# Envia para todos receivers`

O broadcast\!() não é bloqueante, tem um buffer circular de tamanho padrão 1\.

`# Definindo a política na action`  
`@restart{‘always’}`  
`action do_work`  
	`# código que pode falhar`

`@restart{tries: 3, delay: ‘1s’}`  
`action do_another_work`  
	`# código com tentativas limitadas`

`@parallel  # Decorator indicando necessidade de paralelismo`  
`action heavy_compute`  
	`# código que precisa rodar em paralelo`

`action main`  
	`do! do_work()    	# Vai sempre tentar reiniciar`  
	`do! do_another_work() # Vai tentar 3 vezes com delay de 1s`  
	`do! heavy_compute()	# Vai executar, em modo multi processo ou distribuído`

Construções Especiais  
São operações fundamentais da linguagem que:

- Requerem suporte direto do runtime  
- Não podem ser implementadas apenas com as primitivas básicas da linguagem  
- Precisam de gerenciamento de memória ou outras propriedades de sistema

Esses comandos não seguem necessariamente os princípios da linguagem (como log\!, que pode ser chamado em qualquer lugar).

Concorrência:

* `channel!()` \- canal bloqueante  
* `queue!(size)` \- canal com buffer  
* `broadcast!()` \- canal de broadcast, retorna (sender, subscribe(size))  
* `do!` \- execute em modo concorrente (ou paralelo)

I/O:

* `input!(msg)` \- leitura do console  
* `echo!(msg)` \- escrita no console/arquivo  
* `read!(path)` \- leitura de arquivo  
* `log!(msg, lvl)` \- tomada e escrita de log (disponível universalmente)

Outros:

* `now!()` \- timestamp atual  
* `sleep!(duration)` \- pausa execução  
* `panic!(msg)` \- para a execução de forma segura e cria logs  
* `exit!(status)` \- escrita em arquivo

Lista de molduras

Controle de concorrência

`@restart{always} # Para actions que devem sempre tentar reiniciar em caso de falha`  
`@restart{tries: N, delay: Xms} # Para actions com número limitado de tentativas e delay entre elas`  
`@parallel # Para indicar paralelismo`

Controle de cache

`@cache_strategy{size: 1000} # Define tamanho do cache`   
`@cache_strategy{disabled} # Desativa cache para esta função`   
`@cache_strategy{priority: high} # Prioriza manter no cache`

	

Módulos, Imports, Exports e Visibilidade  
        Código confinado a um arquivo configura um módulo.  
        Esse código não é visível a código externo, e não pode acessar código externo a não ser por `imports` e `exports`; os primeiros definem que código externo será visível dentro do escopo local, e os últimos definem que código poderá ser importado por outros módulos.  
       Funções e ações exportadas podem ser usadas fora do seu módulo de origem, tipos exportados levam consigo as funções definidas pelas interfaces que implementam, de forma que, por exemplo, um tipo `Complexo` definido em um módulo, ao ser exportado, carrega consigo as funções cujos nomes estão definidos mas interfaces `ORD` e `NUM` contidas no módulo.

#### Estruturas Base (Todas Imutáveis):

- Tuple: Sequência de tipo homogêneo  
- Option/Union: guarda 1 valor de um conjunto predeterminado de tipos  
- List: Sequência de tipo heterogêneo  
- Dict  
- Set  
- Enum  
- Dataframe: Estrutura com índices únicos, eixos etiquetados \[Imutável\]  
- Vetores e Matrizes (matemáticos)

#### Anotações:

Lembrete de ideias inspiradas em Rust:

- Garantia de ausência de erros durante a execução. Vale a pena? É viável?  
- Enums & sintaxe Match ligados: um match tem que cobrir sempre todas as opções disponíveis\]  
- Match, nesse caso seria específico para Enums? 

#### Ideias em Consideração

- loop/break \<- único laço de repetição, somente em actions

Conteúdos:  
[http://createyourproglang.com](http://createyourproglang.com)  
YACC  
[youtube.com/watch?v=rHCWQ0b3mWg](http://youtube.com/watch?v=rHCWQ0b3mWg)  
[https://youtu.be/HxaD\_trXwRE](https://youtu.be/HxaD_trXwRE)

\- Language implementation patterns  
\- Structure and implementation of computer programs  
\- Livro do dragão

"Practical Foundations for Programming Language" \- Robert Harper

"Lambda Calculus for Computer Scientists" \- Chris Hankin

[https://en.wikibooks.org/wiki/Write\_Yourself\_a\_Scheme\_in\_48\_Hours](https://en.wikibooks.org/wiki/Write_Yourself_a_Scheme_in_48_Hours)

"Real-World Haskell"

Land of Lisp

