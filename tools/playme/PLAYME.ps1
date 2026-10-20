# Requires Windows PowerShell 5.1.
# All generated text is UTF-8 without BOM.
#
# PLAYME genérico do Librarian. Não conhece o domínio do consumidor:
# fontes, léxico, Modelfile, modelo, instruções e critérios vêm do perfil.
#
# Consumidor (perfil) -> Librarian (ingest/grafo/busca) -> E1 conferido
#   -> LLM Engine (prepare/send) -> Ollama -> artefatos e hashes reconferidos.
#
# Fronteiras importantes:
# - O consumidor escolhe fontes, vocabulário, modelo e política de explicação.
# - O Librarian extrai, preserva, verifica, cria o grafo e faz a busca.
# - E1 = hits[0] completo, reconstruído a partir do snapshot preservado.
# - prepare_ollama monta o request exato da API; send_ollama transporta e preserva.
# - A avaliação semântica continua pendente/manual.

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ProfilePath,
    [string]$Question,
    [switch]$PrepareOnly,
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'

# =============================================================================
# Configuração da execução
# =============================================================================

$Utf8 = New-Object Text.UTF8Encoding($false)
[Console]::OutputEncoding = $Utf8

$LibrarianRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path

# Caminhos relativos do perfil são resolvidos a partir da pasta do próprio perfil.
$ProfileFile = (Resolve-Path -LiteralPath $ProfilePath).Path
$ProfileBase = Split-Path -Parent $ProfileFile
$RunProfile = [IO.File]::ReadAllText($ProfileFile, $Utf8) | ConvertFrom-Json

function Resolve-ProfilePath([string]$Field) {
    $Value = $RunProfile.$Field
    if (-not $Value) {
        throw "Perfil sem campo obrigatorio: $Field"
    }
    return [IO.Path]::GetFullPath((Join-Path $ProfileBase $Value))
}

$ProjectRoot = Resolve-ProfilePath 'project_root'
$SourcesConfigPath = Resolve-ProfilePath 'sources_config'
$LexiconPath = Resolve-ProfilePath 'lexicon'
$ModelfilePath = Resolve-ProfilePath 'modelfile'
$InstructionsPath = Resolve-ProfilePath 'instructions'
$CriteriaPath = Resolve-ProfilePath 'criteria'
$OutputRoot = Resolve-ProfilePath 'output_root'
$ModelName = $RunProfile.model
$OllamaBase = $RunProfile.ollama_base_url
$QuestionPrompt = $RunProfile.question_prompt
$SearchTop = [string]$RunProfile.search.top
$SearchDepth = [string]$RunProfile.search.depth
# Quantos primeiros resultados viram fichas (padrao 1 = E1) e linhas extras de contexto.
$SelectHits = if ($null -ne $RunProfile.search.hits) { [string]$RunProfile.search.hits } else { '1' }
$ContextRadius = if ($null -ne $RunProfile.prepare.radius) { [string]$RunProfile.prepare.radius } else { '0' }
# Linhas do contexto como {number, text} com a linha original do arquivo (padrao false).
$NumberedLines = ($null -ne $RunProfile.prepare.numbered_lines) -and [bool]$RunProfile.prepare.numbered_lines
$SendTimeoutMs = [string]$RunProfile.send.timeout_ms
$SendMaxBytes = [string]$RunProfile.send.max_response_bytes
foreach ($Required in @('model', 'ollama_base_url', 'question_prompt')) {
    if (-not $RunProfile.$Required) {
        throw "Perfil sem campo obrigatorio: $Required"
    }
}
foreach ($Number in @($SearchTop, $SearchDepth, $SelectHits, $ContextRadius, $SendTimeoutMs, $SendMaxBytes)) {
    if ($Number -notmatch '^\d+$') {
        throw 'Perfil exige search.top, search.depth, send.timeout_ms e send.max_response_bytes inteiros.'
    }
}
$OllamaHost = ([uri]$OllamaBase).Authority
$OllamaEndpoint = $OllamaBase.TrimEnd('/') + '/api/generate'

$PreviousOllamaHost = $env:OLLAMA_HOST
$script:CommandSequence = 0
$RunStatus = 'failed'
$ExitCode = 1

if (-not $OutputDirectory) {
    $Suffix = '{0}-{1}' -f (Get-Date -Format 'yyyyMMdd-HHmmss'), [guid]::NewGuid().ToString('N').Substring(0, 8)
    $OutputDirectory = Join-Path $OutputRoot "playme-$Suffix"
}

$RunDirectory = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $RunDirectory) {
    throw "Destino existente; escolha outra pasta: $RunDirectory"
}

New-Item -ItemType Directory -Path $RunDirectory | Out-Null
$LogDirectory = New-Item -ItemType Directory -Path (Join-Path $RunDirectory 'logs')
$RunLog = New-Object IO.StreamWriter((Join-Path $RunDirectory 'playme.log'), $false, $Utf8)
$RunLog.AutoFlush = $true

# =============================================================================
# Helpers genéricos
# =============================================================================

function Write-RunLog([string]$Text) {
    Write-Host $Text
    $RunLog.WriteLine($Text)
}

function Save-Artifact([string]$Name, [string]$Text) {
    [IO.File]::WriteAllText((Join-Path $RunDirectory $Name), $Text, $Utf8)
}

function Show-Artifact([string]$Name) {
    Write-RunLog "`n--- $Name ---"
    Write-RunLog ([IO.File]::ReadAllText((Join-Path $RunDirectory $Name), $Utf8))
}

function Get-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function ConvertTo-NativeArgument([string]$Value) {
    # Escaping compatível com CommandLineToArgvW, sem avaliação por shell.
    return '"' + [regex]::Replace(
        [regex]::Replace($Value, '(\\*)"', '$1$1\"'),
        '(\\+)$',
        '$1$1'
    ) + '"'
}

function Invoke-ExternalCommand {
    param(
        [string]$Executable,
        [string[]]$Arguments,
        [int[]]$AllowedExitCodes = @(0)
    )

    $script:CommandSequence++
    $Prefix = '{0:D2}-{1}' -f $script:CommandSequence, [IO.Path]::GetFileNameWithoutExtension($Executable)
    $QuotedArguments = ($Arguments | ForEach-Object { ConvertTo-NativeArgument $_ }) -join ' '

    Write-RunLog "`n> $Executable $QuotedArguments"

    $Info = New-Object Diagnostics.ProcessStartInfo
    $Info.FileName = $Executable
    $Info.Arguments = $QuotedArguments
    $Info.WorkingDirectory = $ProjectRoot
    $Info.UseShellExecute = $false
    $Info.CreateNoWindow = $true
    $Info.RedirectStandardOutput = $true
    $Info.RedirectStandardError = $true
    $Info.StandardOutputEncoding = $Utf8
    $Info.StandardErrorEncoding = $Utf8

    $Process = New-Object Diagnostics.Process
    $Process.StartInfo = $Info

    $OutPath = Join-Path $LogDirectory "$Prefix.stdout.log"
    $ErrPath = Join-Path $LogDirectory "$Prefix.stderr.log"
    $Writers = @(
        (New-Object IO.StreamWriter($OutPath, $false, $Utf8)),
        (New-Object IO.StreamWriter($ErrPath, $false, $Utf8))
    )
    $Started = [DateTime]::UtcNow

    try {
        [void]$Process.Start()

        # Lê stdout e stderr em paralelo para não bloquear o processo filho.
        $Readers = @($Process.StandardOutput, $Process.StandardError)
        $Pending = @($Readers[0].ReadLineAsync(), $Readers[1].ReadLineAsync())

        while ($null -ne $Pending[0] -or $null -ne $Pending[1]) {
            for ($Index = 0; $Index -lt 2; $Index++) {
                if ($null -eq $Pending[$Index] -or -not $Pending[$Index].IsCompleted) {
                    continue
                }

                $Line = $Pending[$Index].GetAwaiter().GetResult()
                if ($null -eq $Line) {
                    $Pending[$Index] = $null
                    continue
                }

                $Writers[$Index].WriteLine($Line)
                $Writers[$Index].Flush()
                Write-RunLog $Line
                $Pending[$Index] = $Readers[$Index].ReadLineAsync()
            }
            Start-Sleep -Milliseconds 10
        }

        $Process.WaitForExit()
        $Code = $Process.ExitCode

        # Fecha os logs antes de reabrir stdout com ReadAllText.
        foreach ($Writer in $Writers) {
            $Writer.Flush()
            $Writer.Dispose()
        }
        $Writers = @()

        $Event = [ordered]@{
            exe = $Executable
            arguments = $Arguments
            exit_code = $Code
            started_utc = $Started.ToString('o')
            finished_utc = [DateTime]::UtcNow.ToString('o')
            stdout = $OutPath
            stderr = $ErrPath
        }
        [IO.File]::AppendAllText(
            (Join-Path $RunDirectory 'commands.jsonl'),
            (($Event | ConvertTo-Json -Compress) + "`n"),
            $Utf8
        )

        Write-RunLog "Exit code: $Code"
        if ($Code -notin $AllowedExitCodes) {
            throw "Comando falhou ($Code): $Executable. Veja $ErrPath"
        }

        return [pscustomobject]@{
            Code = $Code
            Text = [IO.File]::ReadAllText($OutPath, $Utf8)
        }
    }
    finally {
        foreach ($Writer in $Writers) {
            $Writer.Dispose()
        }
        $Process.Dispose()
    }
}

# =============================================================================
# 1. Build dos executáveis
# =============================================================================

function Build-WorkflowTools {
    Write-RunLog 'Etapas 1-2: compilar executaveis e preservar configuracoes.'

    $Cargo = (Get-Command cargo -CommandType Application).Source

    # Librarian: ingestão, conferência, grafo e busca.
    $null = Invoke-ExternalCommand $Cargo @(
        'build', '--locked',
        '--manifest-path', "$LibrarianRoot\Cargo.toml",
        '-p', 'librarian-ingest',
        '--bin', 'librarian_ingest'
    )

    # Librarian: preparação do request e transporte HTTP reutilizáveis.
    $null = Invoke-ExternalCommand $Cargo @(
        'build', '--locked',
        '--manifest-path', "$LibrarianRoot\Cargo.toml",
        '-p', 'librarian-llm-engine',
        '--bin', 'prepare_ollama',
        '--bin', 'send_ollama'
    )

    $LibMeta = (Invoke-ExternalCommand $Cargo @(
        'metadata', '--no-deps', '--format-version', '1',
        '--manifest-path', "$LibrarianRoot\Cargo.toml"
    )).Text | ConvertFrom-Json

    return [pscustomobject]@{
        Ingest = Join-Path $LibMeta.target_directory 'debug\librarian_ingest.exe'
        Prepare = Join-Path $LibMeta.target_directory 'debug\prepare_ollama.exe'
        Send = Join-Path $LibMeta.target_directory 'debug\send_ollama.exe'
    }
}

# =============================================================================
# 2. Consumidor escolhe as fontes; Librarian gera a edição e o grafo
# =============================================================================

function New-LibrarianEdition([string]$IngestExecutable) {
    $Edition = Join-Path $RunDirectory 'edition'
    $Graph = Join-Path $RunDirectory 'graph.json'
    $SourceConfig = Join-Path $RunDirectory 'sources-config.json'
    $Lexicon = Join-Path $RunDirectory 'lexicon.json'

    # Estes arquivos definem o que o consumidor entrega ao Librarian.
    Copy-Item -LiteralPath $SourcesConfigPath -Destination $SourceConfig
    Copy-Item -LiteralPath $LexiconPath -Destination $Lexicon

    Write-RunLog 'Etapas 3-4: acervo, snapshots e grafo.'

    # generate: o Librarian lê as fontes configuradas, extrai estruturas e preserva snapshots.
    $null = Invoke-ExternalCommand $IngestExecutable @('generate', $ProjectRoot, $SourceConfig, $Edition)

    # verify: confere a edição criada contra as fontes do consumidor.
    $null = Invoke-ExternalCommand $IngestExecutable @('verify', $Edition, $ProjectRoot)

    # graph: deriva/exporta relações sintáticas; verify-graph confere a exportação.
    $null = Invoke-ExternalCommand $IngestExecutable @('graph', $Edition, $Graph)
    $null = Invoke-ExternalCommand $IngestExecutable @('verify-graph', $Edition, $Graph)

    # Contrato neutro para o visualizador (tools/graph-viewer); mesma derivacao, conferida.
    $GraphV2 = Join-Path $RunDirectory 'graph-v2.json'
    $null = Invoke-ExternalCommand $IngestExecutable @('graph', $Edition, $GraphV2, 'v2')
    $null = Invoke-ExternalCommand $IngestExecutable @('verify-graph', $Edition, $GraphV2)

    return [pscustomobject]@{
        Edition = $Edition
        Graph = $Graph
        SourceConfig = $SourceConfig
        Lexicon = $Lexicon
    }
}

# =============================================================================
# 3. Librarian busca evidências
# =============================================================================

function Search-LibrarianEvidence {
    param(
        [string]$IngestExecutable,
        [string]$Edition,
        [string]$Lexicon,
        [string]$UserQuestion
    )

    Write-RunLog 'Etapa 5: pergunta, lexico, ranking e relacoes.'

    Save-Artifact 'question.txt' $UserQuestion
    Show-Artifact 'question.txt'

    # search executa internamente:
    # normalização -> expansão léxica -> ranking de símbolos -> expansão limitada do grafo.
    $Search = Invoke-ExternalCommand $IngestExecutable @(
        'search', $Edition, $UserQuestion, $Lexicon, $SearchTop, $SearchDepth
    ) @(0, 2)

    Save-Artifact 'search.json' $Search.Text
    if ($Search.Code -eq 2) {
        $script:RunStatus = 'no_evidence'
        throw 'Sem candidatos ou com diagnosticos; veja search.json. Nenhum envio.'
    }

    $Data = $Search.Text | ConvertFrom-Json

    # Artefatos de inspeção. Eles não entram automaticamente no envelope enviado ao Ollama.
    $LexiconTrace = $Data |
        Select-Object terms, @{ Name = 'matches'; Expression = { $_.hits | Select-Object name, reasons } } |
        ConvertTo-Json -Depth 30
    Save-Artifact 'lexicon-trace.json' $LexiconTrace
    Show-Artifact 'lexicon-trace.json'

    $Excerpts = $Data.hits |
        Format-List name, path, start_line, end_line, score, excerpt_truncated, excerpt |
        Out-String -Width 240
    Save-Artifact 'retrieved-excerpts.txt' $Excerpts
    Show-Artifact 'retrieved-excerpts.txt'

    $Labels = @{}
    foreach ($Node in $Data.graph.nodes) {
        $Labels[$Node.id] = $Node.label
    }

    $Relations = @($Data.graph.edges | ForEach-Object {
        [pscustomobject]@{
            from = $Labels[$_.from]
            kind = $_.kind
            to = $Labels[$_.to]
            resolution = $_.resolution
            path = $_.evidence.path
            line = $_.evidence.start_line
        }
    })

    $GraphTrace = [ordered]@{
        limits = $Data.graph | Select-Object depth, max_nodes, max_edges, truncated, label_bytes_limit, labels_truncated
        relations = $Relations
    } | ConvertTo-Json -Depth 30

    Save-Artifact 'graph-trace.json' $GraphTrace
    Show-Artifact 'graph-trace.json'

    return $Data
}

# =============================================================================
# 4-5. Graph Engine (Rust): busca -> dossie estrutural -> PreparedQuery conferido
# =============================================================================

function New-PreparedQuery {
    param(
        [string]$IngestExecutable,
        [string]$Edition,
        [string]$Lexicon,
        [string]$UserQuestion
    )

    Write-RunLog 'Etapas 6-7: dossie estrutural dos primeiros resultados, PreparedQuery e criterios previos.'

    # Pedido autocontido: reaplicado sobre a mesma edition, reproduz todos os bytes.
    # PowerShell 5.1 emite o array JSON como um objeto; os parênteses o enumeram.
    $Instructions = @(([IO.File]::ReadAllText($InstructionsPath, $Utf8) | ConvertFrom-Json) | ForEach-Object { [string]$_ })
    $Request = [ordered]@{
        question = $UserQuestion
        lexicon = ([IO.File]::ReadAllText($Lexicon, $Utf8) | ConvertFrom-Json)
        top = [int]$SearchTop
        depth = [int]$SearchDepth
        hits = [int]$SelectHits
        radius = [int]$ContextRadius
        numbered_lines = $NumberedLines
        instructions = $Instructions
    }
    Save-Artifact 'prepare-request.json' ($Request | ConvertTo-Json -Depth 10)

    # prepare: refaz a busca, monta fichas estruturais, confere fontes contra snapshots,
    # exporta o bundle e o reconfere a partir do disco.
    $Prepared = Join-Path $RunDirectory 'prepared'
    $null = Invoke-ExternalCommand $IngestExecutable @(
        'prepare', $Edition, (Join-Path $RunDirectory 'prepare-request.json'), $Prepared
    )

    Show-Artifact 'prepared\bundle\dossier.json'
    Show-Artifact 'prepared\bundle\query.json'

    # Criterios do consumidor, preservados antes de qualquer resposta.
    Save-Artifact 'criteria-before.md' ([IO.File]::ReadAllText($CriteriaPath, $Utf8))
    Show-Artifact 'criteria-before.md'

    return [pscustomobject]@{
        Directory = $Prepared
        Query = Join-Path $Prepared 'bundle\query.json'
    }
}

# =============================================================================
# 6. Ollama: servidor + modelo configurado pelo Modelfile
# =============================================================================

function Initialize-Ollama {
    Write-RunLog 'Etapa 7: servidor Ollama local e Modelfile.'

    $Ollama = (Get-Command ollama -CommandType Application).Source
    $env:OLLAMA_HOST = $OllamaHost

    $Ready = $false
    $Tags = $null

    try {
        $Tags = Invoke-RestMethod ($OllamaBase.TrimEnd('/') + '/api/tags') -TimeoutSec 2
        $Ready = $true
    }
    catch {
        Write-RunLog 'Iniciando ollama serve; stdout/stderr em logs/server-*.'
    }

    if (-not $Ready) {
        $StartArgs = @{
            FilePath = $Ollama
            ArgumentList = 'serve'
            WindowStyle = 'Hidden'
            RedirectStandardOutput = (Join-Path $LogDirectory 'server-stdout.log')
            RedirectStandardError = (Join-Path $LogDirectory 'server-stderr.log')
            PassThru = $true
        }
        $Server = Start-Process @StartArgs

        Save-Artifact 'server.json' (([ordered]@{
            pid = $Server.Id
            started_by_playme = $true
            endpoint = $OllamaBase
        }) | ConvertTo-Json)

        for ($Attempt = 0; $Attempt -lt 30; $Attempt++) {
            try {
                $Tags = Invoke-RestMethod ($OllamaBase.TrimEnd('/') + '/api/tags') -TimeoutSec 1
                $Ready = $true
                break
            }
            catch {
                Start-Sleep -Seconds 1
            }

            if ($Server.HasExited) {
                throw 'ollama serve encerrou. Veja logs/server-stderr.log.'
            }
        }

        if (-not $Ready) {
            throw 'Servidor nao ficou pronto; consulte logs/server-*.'
        }

        Write-RunLog "Servidor iniciado, PID $($Server.Id). Permanecera disponivel apos a consulta."
    }
    else {
        Write-RunLog 'Servidor existente reutilizado.'
    }

    Save-Artifact 'server-tags.json' ($Tags | ConvertTo-Json -Depth 20)
    Show-Artifact 'server-tags.json'

    $RequestedModelfile = Join-Path $RunDirectory 'Modelfile.requested'
    Copy-Item -LiteralPath $ModelfilePath -Destination $RequestedModelfile
    Show-Artifact 'Modelfile.requested'

    $ModelfileText = [IO.File]::ReadAllText($RequestedModelfile)
    $BaseModel = [regex]::Match($ModelfileText, '(?im)^FROM\s+(\S+)\s*$').Groups[1].Value
    if (-not $BaseModel) {
        throw 'FROM ausente no Modelfile.'
    }

    # Editar o Modelfile não aplica a configuração; o modelo é materializado com create.
    $null = Invoke-ExternalCommand $Ollama @('pull', $BaseModel)
    $null = Invoke-ExternalCommand $Ollama @('create', $ModelName, '-f', $RequestedModelfile)

    Save-Artifact 'model-parameters.txt' (Invoke-ExternalCommand $Ollama @('show', $ModelName, '--parameters')).Text
    Save-Artifact 'model-modelfile.txt' (Invoke-ExternalCommand $Ollama @('show', $ModelName, '--modelfile')).Text
}

# =============================================================================
# 7. prepare_ollama: query -> request exato da API
# =============================================================================

function Prepare-OllamaRequest([string]$PrepareExecutable, [string]$QueryPath) {
    Write-RunLog 'Etapa 8: preparar corpo exato do prompt para a API.'

    $RequestPath = Join-Path $RunDirectory 'request-ollama.json'
    $null = Invoke-ExternalCommand $PrepareExecutable @($QueryPath, $ModelName, $RequestPath)
    Show-Artifact 'request-ollama.json'
}

# =============================================================================
# 8. send_ollama: transporte + preservação + conferência dos hashes
# =============================================================================

function Send-OllamaAttempt([string]$SendExecutable, [string]$QueryPath) {
    $RequestId = 'PLAYME_' + [guid]::NewGuid().ToString('N')
    $AttemptDirectory = Join-Path $RunDirectory 'ollama-01'

    $Sent = Invoke-ExternalCommand $SendExecutable @(
        $QueryPath,
        $OllamaEndpoint,
        $ModelName,
        $SendTimeoutMs,
        $SendMaxBytes,
        $RequestId,
        $AttemptDirectory
    ) @(0, 1, 2)

    Write-RunLog 'Etapa 9: requisicao efetiva, resposta, estado e hashes.'

    foreach ($Name in @('query.json', 'request.json', 'response.bin', 'response.txt', 'result.json')) {
        if (Test-Path -LiteralPath (Join-Path $AttemptDirectory $Name)) {
            Show-Artifact "ollama-01\$Name"
        }
    }

    $ResultPath = Join-Path $AttemptDirectory 'result.json'
    if (-not (Test-Path -LiteralPath $ResultPath)) {
        throw 'Tentativa parcial: result.json ausente.'
    }
    $Result = Get-Content -Raw -Encoding UTF8 $ResultPath | ConvertFrom-Json

    # Recalcula os hashes dos artefatos preservados; não confia apenas em result.json.
    $Checks = @(
        @('query.json', 'query_sha256'),
        @('request.json', 'request_sha256'),
        @('response.bin', 'response_sha256'),
        @('response.txt', 'text_sha256')
    )

    foreach ($Check in $Checks) {
        $FileName = $Check[0]
        $HashField = $Check[1]
        $Expected = $Result.$HashField

        if ($Expected) {
            $Actual = Get-Sha256 (Join-Path $AttemptDirectory $FileName)
            if ($Actual -cne $Expected) {
                throw "Hash divergente: $FileName"
            }
            Write-RunLog "SHA256 OK: $FileName $Actual"
        }
        elseif ($Sent.Code -eq 0) {
            throw "Hash ausente: $FileName"
        }
    }

    # O script prepara a avaliação, mas não promove a resposta a fato nem a avalia sozinho.
    $Evaluation = @"
# Avaliacao pendente
Request: $RequestId
Hash texto: $($Result.text_sha256)
Estado transporte: $($Result.state)

Preencha cada criterio de criteria-before.md com citacoes da resposta. Nao altere os originais.
"@
    Save-Artifact 'evaluation.md' $Evaluation

    return [pscustomobject]@{
        State = $Result.state
        ExitCode = $Sent.Code
    }
}

# =============================================================================
# Workflow principal
# =============================================================================

try {
    Write-RunLog "PLAYME - perfil $ProfileFile. Artefatos e logs: $RunDirectory"

    # 1) Compila os executáveis usados nas fronteiras.
    $Tools = Build-WorkflowTools

    # 2) Consumidor escolhe as fontes; Librarian extrai/preserva/verifica e cria o grafo.
    $Edition = New-LibrarianEdition $Tools.Ingest

    # 3) Librarian recebe a pergunta e executa normalização, léxico, ranking e expansão.
    if (-not $Question) {
        $Question = Read-Host $QuestionPrompt
    }
    if ([string]::IsNullOrWhiteSpace($Question)) {
        throw 'Pergunta vazia.'
    }

    $SearchArgs = @{
        IngestExecutable = $Tools.Ingest
        Edition = $Edition.Edition
        Lexicon = $Edition.Lexicon
        UserQuestion = $Question
    }
    # Artefatos de inspecao (ranking, trechos, vizinhanca); a selecao e refeita pelo prepare.
    $null = Search-LibrarianEvidence @SearchArgs

    # 4-5) Graph Engine em Rust: dossie, PreparedQuery conferido e criterios previos.
    $PreparedArgs = @{
        IngestExecutable = $Tools.Ingest
        Edition = $Edition.Edition
        Lexicon = $Edition.Lexicon
        UserQuestion = $Question
    }
    $Prepared = New-PreparedQuery @PreparedArgs
    $QueryPath = $Prepared.Query

    # 6) Em PrepareOnly não inicia servidor/modelo, mas ainda gera o request exato.
    if (-not $PrepareOnly) {
        Initialize-Ollama
    }

    # 7) prepare_ollama transforma a query no corpo exato da API.
    Prepare-OllamaRequest $Tools.Prepare $QueryPath

    if ($PrepareOnly) {
        $RunStatus = 'prepared_only'
        $ExitCode = 0
        Write-RunLog 'PrepareOnly: prompt/request prontos. Servidor, modelo e envio nao executados.'
    }
    else {
        # 8) send_ollama envia, preserva a tentativa e PLAYME reconfere os hashes.
        # Reconfere o bundle contra a edition imediatamente antes do envio.
        $null = Invoke-ExternalCommand $Tools.Ingest @('verify-prepared', $Edition.Edition, $Prepared.Directory)
        $Attempt = Send-OllamaAttempt $Tools.Send $QueryPath
        $RunStatus = $Attempt.State
        $ExitCode = $Attempt.ExitCode
        Write-RunLog "Transporte: $RunStatus. Avaliacao semantica PENDENTE (evaluation.md)."
    }
}
catch {
    Write-RunLog ("ERRO: " + $_.Exception.Message)
    Write-RunLog $_.ScriptStackTrace
    $ExitCode = 1
}
finally {
    # Restaura o ambiente do chamador mesmo quando uma etapa falha.
    $env:OLLAMA_HOST = $PreviousOllamaHost

    Save-Artifact 'run.json' (([ordered]@{
        status = $RunStatus
        exit_code = $ExitCode
        prepare_only = [bool]$PrepareOnly
        semantic_evaluation = 'pending'
        finished_utc = [DateTime]::UtcNow.ToString('o')
        directory = $RunDirectory
    }) | ConvertTo-Json)

    Write-RunLog "`nLogs e artefatos preservados: $RunDirectory"
    Write-RunLog ("Visualizar: abra " + (Join-Path $LibrarianRoot 'tools\graph-viewer\index.html') + " e escolha esta pasta.")
    $RunLog.Dispose()
}

exit $ExitCode
