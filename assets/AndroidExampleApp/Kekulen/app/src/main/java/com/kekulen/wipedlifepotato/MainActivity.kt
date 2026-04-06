package com.kekulen.wipedlifepotato

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kekulen.wipedlifepotato.ui.theme.KekulenTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val binPath = "${applicationInfo.nativeLibraryDir}/libkekulen.so"
        val workDir = filesDir
        kotlin.concurrent.thread {
            try {
                val file = File(binPath)
                if (file.exists()) {
                    file.setExecutable(true)
                    ProcessBuilder(binPath).directory(workDir).start()
                }
            } catch (_: Exception) { }
        }

        enableEdgeToEdge()
        setContent {
            KekulenTheme {
                KekulenApp()
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KekulenApp() {
    val scope = rememberCoroutineScope()
    val drawerState = rememberDrawerState(DrawerValue.Closed)
    val clipboardManager = LocalClipboardManager.current
    val listState = rememberLazyListState()

    var myKey by remember { mutableStateOf("Loading...") }
    var friendsList by remember { mutableStateOf(listOf<JSONObject>()) }
    var currentChat by remember { mutableStateOf<String?>(null) }
    var inputText by remember { mutableStateOf("") }

    // Состояния для диалога добавления друга
    var showAddDialog by remember { mutableStateOf(false) }
    var newFriendName by remember { mutableStateOf("") }
    var newFriendKey by remember { mutableStateOf("") }

    LaunchedEffect(Unit) {
        while(true) {
            withContext(Dispatchers.IO) {
                try {
                    val me = URL("http://127.0.0.1:8080/api/me").readText()
                    myKey = JSONObject(me).getString("pub_key")

                    val fr = URL("http://127.0.0.1:8080/api/friends").readText()
                    val arr = JSONArray(fr)
                    val list = mutableListOf<JSONObject>()
                    for (i in 0 until arr.length()) { list.add(arr.getJSONObject(i)) }
                    friendsList = list
                } catch (_: Exception) { }
            }
            delay(2000)
        }
    }

    // Функция отправки сообщения
    fun sendMessage() {
        val chat = currentChat ?: return
        val text = inputText
        if (text.isBlank()) return
        scope.launch(Dispatchers.IO) {
            try {
                val conn = URL("http://127.0.0.1:8080/api/send/$chat").openConnection() as HttpURLConnection
                conn.requestMethod = "POST"
                conn.doOutput = true
                conn.outputStream.write(JSONObject().put("message", text).toString().toByteArray())
                conn.responseCode
                withContext(Dispatchers.Main) { inputText = "" }
            } catch (_: Exception) { }
        }
    }

    // Функция добавления друга
    fun addFriend() {
        if (newFriendName.isBlank() || newFriendKey.isBlank()) return
        scope.launch(Dispatchers.IO) {
            try {
                val conn = URL("http://127.0.0.1:8080/api/friends/add").openConnection() as HttpURLConnection
                conn.requestMethod = "POST"
                conn.doOutput = true
                val body = JSONObject().put("name", newFriendName).put("pub_key", newFriendKey).toString()
                conn.outputStream.write(body.toByteArray())
                conn.responseCode
                withContext(Dispatchers.Main) {
                    showAddDialog = false
                    newFriendName = ""
                    newFriendKey = ""
                }
            } catch (_: Exception) { }
        }
    }

    // Диалоговое окно добавления друга
    if (showAddDialog) {
        AlertDialog(
            onDismissRequest = { showAddDialog = false },
            containerColor = Color(0xFF1E1E1E),
            title = { Text("Add Friend", color = Color.White) },
            text = {
                Column {
                    TextField(
                        value = newFriendName,
                        onValueChange = { newFriendName = it },
                        placeholder = { Text("Name") },
                        modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp)
                    )
                    TextField(
                        value = newFriendKey,
                        onValueChange = { newFriendKey = it },
                        placeholder = { Text("I2P Destination (Key)") },
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            },
            confirmButton = {
                TextButton(onClick = { addFriend() }) { Text("ADD", color = Color(0xFF00FF9D)) }
            },
            dismissButton = {
                TextButton(onClick = { showAddDialog = false }) { Text("CANCEL", color = Color.Gray) }
            }
        )
    }

    ModalNavigationDrawer(
        drawerState = drawerState,
        drawerContent = {
            ModalDrawerSheet(
                drawerContainerColor = Color(0xFF121212),
                modifier = Modifier.width(300.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(16.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("Kekulen PQC", color = Color(0xFF00FF9D), fontWeight = FontWeight.Bold)
                    // КНОПКА ДОБАВЛЕНИЯ [+]
                    Text(
                        "[ + ]",
                        color = Color(0xFF00FF9D),
                        modifier = Modifier.clickable { showAddDialog = true },
                        fontWeight = FontWeight.Bold
                    )
                }

                Column(modifier = Modifier.fillMaxWidth().clickable { clipboardManager.setText(AnnotatedString(myKey)) }.padding(horizontal = 16.dp)) {
                    Text("Your ID (Tap to copy):", color = Color.Gray, fontSize = 10.sp)
                    Text(myKey.take(32) + "...", color = Color.LightGray, fontSize = 11.sp)
                }

                Spacer(Modifier.height(16.dp))
                HorizontalDivider(color = Color.DarkGray)

                LazyColumn {
                    items(friendsList) { friend ->
                        val pk = friend.getString("pub_key")
                        ListItem(
                            headlineContent = { Text(friend.getString("name"), color = Color.White) },
                            modifier = Modifier.clickable {
                                currentChat = pk
                                scope.launch { drawerState.close() }
                            },
                            colors = ListItemDefaults.colors(containerColor = Color.Transparent)
                        )
                    }
                }
            }
        }
    ) {
        Scaffold(
            containerColor = Color(0xFF121212),
            topBar = {
                TopAppBar(
                    title = { Text("Kekulen", color = Color.White) },
                    navigationIcon = {
                        Text(" MENU ", color = Color(0xFF00FF9D), modifier = Modifier.padding(8.dp).clickable { scope.launch { drawerState.open() } }, fontWeight = FontWeight.Bold)
                    },
                    colors = TopAppBarDefaults.topAppBarColors(containerColor = Color(0xFF1E1E1E))
                )
            },
            bottomBar = {
                if (currentChat != null) {
                    Surface(color = Color(0xFF1E1E1E)) {
                        Row(Modifier.padding(12.dp).navigationBarsPadding().imePadding(), verticalAlignment = Alignment.CenterVertically) {
                            TextField(
                                value = inputText,
                                onValueChange = { inputText = it },
                                modifier = Modifier.weight(1f),
                                shape = RoundedCornerShape(24.dp),
                                colors = TextFieldDefaults.colors(focusedContainerColor = Color(0xFF121212), unfocusedContainerColor = Color(0xFF121212), focusedTextColor = Color.White, unfocusedTextColor = Color.White, focusedIndicatorColor = Color.Transparent, unfocusedIndicatorColor = Color.Transparent)
                            )
                            Spacer(Modifier.width(8.dp))
                            Box(modifier = Modifier.size(48.dp).background(Color(0xFF00FF9D), CircleShape).clickable { sendMessage() }, contentAlignment = Alignment.Center) {
                                Text(">>>", color = Color(0xFF121212), fontWeight = FontWeight.Bold)
                            }
                        }
                    }
                }
            }
        ) { p ->
            Box(Modifier.padding(p).fillMaxSize()) {
                if (currentChat == null) {
                    Text("Выберите чат в меню", color = Color.Gray, modifier = Modifier.align(Alignment.Center))
                } else {
                    val activeFriend = friendsList.find { it.getString("pub_key") == currentChat }
                    val messagesArray = activeFriend?.getJSONArray("messages") ?: JSONArray()

                    LaunchedEffect(messagesArray.length()) {
                        if (messagesArray.length() > 0) listState.animateScrollToItem(messagesArray.length() - 1)
                    }

                    LazyColumn(state = listState, modifier = Modifier.fillMaxSize().padding(horizontal = 16.dp)) {
                        items((0 until messagesArray.length()).toList()) { i ->
                            val msg = messagesArray.getString(i)
                            val isMe = msg.startsWith("Me: ")
                            val displayText = msg.replace("Me: ", "")

                            Column(Modifier.fillMaxWidth().padding(vertical = 4.dp), horizontalAlignment = if (isMe) Alignment.End else Alignment.Start) {
                                Surface(
                                    color = if (isMe) Color(0xFF005C4B) else Color(0xFF333333),
                                    shape = RoundedCornerShape(topStart = 12.dp, topEnd = 12.dp, bottomStart = if (isMe) 12.dp else 2.dp, bottomEnd = if (isMe) 2.dp else 12.dp)
                                ) {
                                    Text(displayText, color = Color.White, modifier = Modifier.padding(12.dp))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}