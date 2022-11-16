function fetchProduct(){
	const { invoke } = window.__TAURI__.tauri 

	invoke('get_product', { lpn: document.getElementById("lpnInput").value })
		.then((data) =>{
			if (data != null){
				document.getElementById("productName").innerHTML = "Name: " + data[0];
				document.getElementById("productImage").setAttribute("src", data[1]);
				document.getElementById("productDescription").innerHTML = "Description: " + data[2];
				document.getElementById("productMsrp").innerHTML = "MSRP: " + data[3];
			}
			else{
				document.getElementById("productName").innerHTML = "Name: None";
				document.getElementById("productImage").setAttribute("src", "https://i.guim.co.uk/img/media/fe1e34da640c5c56ed16f76ce6f994fa9343d09d/0_174_3408_2046/master/3408.jpg?width=1200&height=1200&quality=85&auto=format&fit=crop&s=67773a9d419786091c958b2ad08eae5e");
				document.getElementById("productDescription").innerHTML = "Description: None";
				document.getElementById("productMsrp").innerHTML = "MSRP: None";
			}
	})
}
